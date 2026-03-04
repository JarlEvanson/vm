//! Implementation of the setup for cross address space switching.

use core::{
    error, fmt,
    mem::{self, MaybeUninit},
    ptr, slice,
};

use conversion::{u64_to_usize_strict, usize_to_u64};
use memory::{
    address::{AddressChunk, AddressChunkRange, PhysicalAddress, PhysicalAddressRange},
    translation::{MapError, MapFlags, TranslationScheme},
};
use stub_api::{
    GenericTable, Header,
    raw::{GenericTable32, GenericTable64},
};
use sync::{Spinlock, SpinlockGuard};

use crate::{
    arch::{
        paging::ArchScheme,
        switch::{
            ArchCodeLayout, CpuStorage, allocate_code, arch_table_64_bit, arch_table_size,
            base_cpu_storage, finalize_cpu_data, handle_stack_allocation,
            handle_storage_allocation, write_protocol_table_32, write_protocol_table_64,
        },
    },
    platform::{
        Allocation, AllocationPolicy, FrameAllocation, OutOfMemory, allocate,
        allocate_frames_aligned, cpu_count, flags, frame_size, main_processor_id, map_identity,
    },
    util::DropWrapper,
};

/// The size, in bytes, of a CPU's stack.
const STACK_SIZE: u64 = 16 * 1024;

/// The data required to handle the cross-address space calls.
static SWITCH_DATA: Spinlock<Option<SwitchData>> = Spinlock::new(None);

/// Returns the contents of [`CpuData`] slice and the provided [`ArchScheme`].
pub fn switch_data() -> SpinlockGuard<'static, Option<SwitchData>> {
    SWITCH_DATA.lock()
}

/// Clears the stored [`CpuData`] slice and provided [`ArchScheme`].
pub fn clear() {
    *SWITCH_DATA.lock() = None;
}

/// Allocates and maps information required for all CPUs to carry out cross address space function
/// calls.
#[expect(clippy::missing_errors_doc)]
pub fn setup(mut scheme: ArchScheme, policy: AllocationPolicy) -> Result<(), CpuDataError> {
    let base_storage = base_cpu_storage(&mut scheme);

    let cpu_data_size = u64_to_usize_strict(
        cpu_count().strict_mul(usize_to_u64(mem::size_of::<Spinlock<CpuData>>())),
    );
    let cpu_data_array =
        allocate(cpu_data_size, mem::align_of::<Spinlock<CpuData>>()).ok_or_else(|| todo!())?;

    // SAFETY:
    //
    // The slice was properly allocated and MaybeUninit does not expect initialization.
    let cpu_data_slice = unsafe {
        slice::from_raw_parts_mut(
            cpu_data_array
                .ptr()
                .cast::<MaybeUninit<Spinlock<CpuData>>>(),
            u64_to_usize_strict(cpu_count()),
        )
    };

    let mut wrapper = DropWrapper {
        val: (cpu_data_array, cpu_data_slice, 0usize),
        drop_func: |(_, cpu_data_slice, init_count)| {
            for (_, cpu_data) in (0..*init_count).zip(cpu_data_slice.iter_mut()) {
                let ptr = ptr::from_mut(cpu_data).cast::<Spinlock<CpuData>>();
                // SAFETY:
                //
                // The data has been initialized.
                unsafe { ptr::drop_in_place(ptr) }
            }
        },
    };

    for maybe_uninit_cpu_data in wrapper.val.1.iter_mut() {
        let cpu_data = allocate_cpu_data(&mut scheme, base_storage, policy)?;
        maybe_uninit_cpu_data.write(Spinlock::new(cpu_data));
        wrapper.val.2 += 1;
    }

    let (cpu_data_allocation, _, _) = wrapper.into_inner();
    let switch_data = SwitchData {
        scheme,
        cpu_data: cpu_data_allocation,
    };

    *SWITCH_DATA.lock() = Some(switch_data);
    Ok(())
}

/// Allocates and maps all of the information required for a CPU to carry out cross address space
/// function calls.
fn allocate_cpu_data(
    scheme: &mut ArchScheme,
    mut base_storage: CpuStorage,
    policy: AllocationPolicy,
) -> Result<CpuData, CpuDataError> {
    let storage_frame_allocation =
        allocate_storage(scheme, &mut base_storage, policy).map_err(CpuDataError::Storage)?;

    let stack_frame_allocation =
        allocate_stack(scheme, &mut base_storage, policy).map_err(CpuDataError::Stack)?;

    let storage_pointer = storage_frame_allocation
        .range()
        .start()
        .start_address(frame_size())
        .value();

    let stub_code_layout = allocate_code(scheme, storage_pointer, &mut base_storage, true)
        .map_err(CpuDataError::StubCode)?;
    let executable_code_layout = allocate_code(scheme, storage_pointer, &mut base_storage, false)
        .map_err(CpuDataError::StubCode)?;

    let mut cpu_data = CpuData {
        storage: storage_frame_allocation,
        stack: stack_frame_allocation,
        stub: stub_code_layout,
        executable: executable_code_layout,
    };

    finalize_cpu_data(&mut cpu_data, &mut base_storage);
    *cpu_data.storage_mut() = base_storage;
    Ok(cpu_data)
}

/// Allocates and maps a stack in the executable's address space for the CPU associated with the
/// provided [`CpuStorage`].
fn allocate_stack(
    scheme: &mut ArchScheme,
    storage: &mut CpuStorage,
    policy: AllocationPolicy,
) -> Result<FrameAllocation, ComponentError> {
    let stack_pages = STACK_SIZE.div_ceil(scheme.chunk_size());
    let frame_allocation = allocate_frames_aligned(
        STACK_SIZE.div_ceil(frame_size()),
        scheme.chunk_size(),
        policy,
    )?;

    let frame_range = AddressChunkRange::new(
        AddressChunk::containing_address(
            frame_allocation
                .range()
                .start()
                .start_address(frame_size())
                .to_address(),
            scheme.chunk_size(),
        ),
        stack_pages,
    );

    // SAFETY:
    //
    // `arch_scheme` is not actively in use.
    unsafe {
        scheme.map(frame_range, frame_range, MapFlags::READ | MapFlags::WRITE)?;
    }

    let stack_top = frame_range
        .start()
        .start_address(scheme.chunk_size())
        .value()
        + STACK_SIZE;
    handle_stack_allocation(storage, stack_top);
    Ok(frame_allocation)
}

/// Allocates and maps a region of memory in which the provided [`CpuStorage`] will be placed.
fn allocate_storage(
    scheme: &mut ArchScheme,
    storage: &mut CpuStorage,
    policy: AllocationPolicy,
) -> Result<FrameAllocation, ComponentError> {
    let storage_size = mem::size_of::<CpuStorage>();
    let storage_size_u64 = usize_to_u64(storage_size);

    let frame_allocation = allocate_frames_aligned(
        storage_size_u64.div_ceil(frame_size()),
        scheme.chunk_size(),
        policy,
    )?;

    let frame_range = AddressChunkRange::new(
        AddressChunk::containing_address(
            frame_allocation
                .range()
                .start()
                .start_address(frame_size())
                .to_address(),
            scheme.chunk_size(),
        ),
        storage_size_u64.div_ceil(scheme.chunk_size()),
    );

    let _ = map_identity(PhysicalAddressRange::new(
        frame_allocation.range().start().start_address(frame_size()),
        storage_size_u64,
    ));

    // SAFETY:
    //
    // `arch_scheme` is not actively in use.
    unsafe {
        scheme.map(frame_range, frame_range, MapFlags::READ | MapFlags::WRITE)?;
    }

    let storage_base = frame_allocation
        .range()
        .start()
        .start_address(frame_size())
        .value();
    handle_storage_allocation(storage, storage_base);
    Ok(frame_allocation)
}

/// Allocates and maps the protocol table.
#[expect(clippy::missing_errors_doc)]
#[expect(clippy::missing_panics_doc)]
pub fn allocate_protocol_table(
    scheme: &mut ArchScheme,
    layout: &CodeLayout,
    image_physical_address: PhysicalAddress,
    image_virtual_address: u64,
) -> Result<FrameAllocation, ComponentError> {
    let total_size = arch_table_size(scheme);
    let total_size_u64 = usize_to_u64(total_size);

    let frame_allocation = allocate_frames_aligned(
        total_size_u64.div_ceil(frame_size()),
        scheme.chunk_size(),
        AllocationPolicy::Below(scheme.output_descriptor().valid_ranges()[0].1),
    )?;

    let frame_range = AddressChunkRange::new(
        AddressChunk::containing_address(
            frame_allocation
                .range()
                .start()
                .start_address(frame_size())
                .to_address(),
            scheme.chunk_size(),
        ),
        total_size_u64.div_ceil(scheme.chunk_size()),
    );

    // SAFETY:
    //
    // `arch_scheme` is not actively in use.
    unsafe {
        scheme.map(frame_range, frame_range, MapFlags::READ | MapFlags::WRITE)?;
    }

    let address = frame_allocation.range().start().start_address(frame_size());
    if arch_table_64_bit(scheme) {
        // 64-bit address space.

        let protocol_table = RevmProtocolTable64 {
            header: Header {
                version: Header::VERSION,
                last_major_version: Header::LAST_MAJOR_VERSION,
                length: usize_to_u64(mem::size_of::<RevmProtocolTable64<()>>()),
                generic_table_offset: usize_to_u64(mem::offset_of!(
                    RevmProtocolTable64<()>,
                    generic_table
                )),
                arch_table_offset: usize_to_u64(mem::offset_of!(
                    RevmProtocolTable64<()>,
                    arch_table
                )),
            },
            generic_table: GenericTable64 {
                version: GenericTable::VERSION,
                page_frame_size: frame_size().max(scheme.chunk_size()),
                image_physical_address: image_physical_address.value(),
                image_virtual_address,
                main_cpu: main_processor_id(),
                cpu_count: cpu_count(),
                flags: flags(),
                write: layout.write,
                allocate_frames: layout.allocate_frames,
                deallocate_frames: layout.deallocate_frames,
                get_memory_map: layout.get_memory_map,
                map: layout.map,
                unmap: layout.unmap,
                takeover: layout.takeover,
                run_on_all_processors: layout.run_on_all_processors,
            },
            arch_table: (),
        };

        write_protocol_table_64(protocol_table, address);
    } else {
        // 32-bit address space.

        let protocol_table = RevmProtocolTable32 {
            header: Header {
                version: Header::VERSION,
                last_major_version: Header::LAST_MAJOR_VERSION,
                length: usize_to_u64(mem::size_of::<RevmProtocolTable32<()>>()),
                generic_table_offset: usize_to_u64(mem::offset_of!(
                    RevmProtocolTable32<()>,
                    generic_table
                )),
                arch_table_offset: usize_to_u64(mem::offset_of!(
                    RevmProtocolTable32<()>,
                    arch_table
                )),
            },
            generic_table: GenericTable32 {
                version: GenericTable::VERSION,
                page_frame_size: frame_size().max(scheme.chunk_size()),
                image_physical_address: image_physical_address.value(),
                image_virtual_address,
                main_cpu: main_processor_id(),
                cpu_count: cpu_count(),
                flags: flags(),
                write: u32::try_from(layout.write).expect("failed to convert function to u32"),
                allocate_frames: u32::try_from(layout.allocate_frames)
                    .expect("failed to convert function to u32"),
                deallocate_frames: u32::try_from(layout.deallocate_frames)
                    .expect("failed to convert function to u32"),
                get_memory_map: u32::try_from(layout.get_memory_map)
                    .expect("failed to convert function to u32"),
                map: u32::try_from(layout.map).expect("failed to convert function to u32"),
                unmap: u32::try_from(layout.unmap).expect("failed to convert function to u32"),
                takeover: u32::try_from(layout.takeover)
                    .expect("failed to convert function to u32"),
                run_on_all_processors: u32::try_from(layout.run_on_all_processors)
                    .expect("failed to convert function to u32"),
            },
            arch_table: (),
        };

        write_protocol_table_32(protocol_table, address);
    }

    Ok(frame_allocation)
}

/// Important data needed to resolve cross address space calls.
pub struct SwitchData {
    /// The [`TranslationScheme`] used for mapping and translation.
    scheme: ArchScheme,
    /// The allocation that contains the [`CpuData`] array.
    cpu_data: Allocation,
}

impl SwitchData {
    /// Returns an immutable reference to the [`ArchScheme`].
    pub fn scheme(&self) -> &ArchScheme {
        self.both_ref().0
    }

    /// Returns a mutable reference to the [`ArchScheme`].
    pub fn scheme_mut(&mut self) -> &mut ArchScheme {
        self.both_mut().0
    }

    /// Returns an immutable slice of the [`CpuData`].
    pub fn cpu_data(&self) -> &[Spinlock<CpuData>] {
        self.both_ref().1
    }

    /// Returns a mutable slice of the [`CpuData`].
    pub fn cpu_data_mut(&mut self) -> &mut [Spinlock<CpuData>] {
        self.both_mut().1
    }

    /// Returns mutable references to the [`ArchScheme`] and [`CpuData`] array.
    pub fn both_ref(&self) -> (&ArchScheme, &[Spinlock<CpuData>]) {
        let scheme = &self.scheme;

        // SAFETY:
        //
        // The slice was properly allocated and [`CpuData`] has been initialized.
        let slice = unsafe {
            slice::from_raw_parts(
                self.cpu_data.ptr().cast::<Spinlock<CpuData>>(),
                u64_to_usize_strict(cpu_count()),
            )
        };

        (scheme, slice)
    }

    /// Returns mutable references to the [`ArchScheme`] and [`CpuData`] array.
    pub fn both_mut(&mut self) -> (&mut ArchScheme, &mut [Spinlock<CpuData>]) {
        let scheme = &mut self.scheme;

        // SAFETY:
        //
        // The slice was properly allocated and [`CpuData`] has been initialized.
        let slice = unsafe {
            slice::from_raw_parts_mut(
                self.cpu_data.ptr().cast::<Spinlock<CpuData>>(),
                u64_to_usize_strict(cpu_count()),
            )
        };

        (scheme, slice)
    }
}

impl Drop for SwitchData {
    fn drop(&mut self) {
        let base_ptr = self.cpu_data.ptr().cast::<Spinlock<CpuData>>();
        for i in 0..u64_to_usize_strict(cpu_count()) {
            let ptr = base_ptr.wrapping_add(i);
            // SAFETY:
            //
            // Drop each and every [`CpuData`] that was stored in the [`Allocation`].
            unsafe { ptr::drop_in_place(ptr) }
        }
    }
}

/// Information required for a CPU to carry out a cross address space function call.
#[derive(Debug)]
pub struct CpuData {
    /// The [`CpuStorage`] associated with the CPU.
    pub storage: FrameAllocation,
    /// The stack associated with the CPU.
    pub stack: FrameAllocation,
    /// The layout of the stub's code associated with the CPU.
    pub stub: CodeLayout,
    /// The layout of the executable's code associated with the CPU.
    pub executable: CodeLayout,
}

impl CpuData {
    /// Returns a pointer that refers to the location of the [`CpuStorage`] associated with this
    /// [`CpuData`].
    fn storage_ptr(&self) -> *mut CpuStorage {
        self.storage
            .range()
            .start()
            .start_address(frame_size())
            .value() as *mut CpuStorage
    }

    /// Returns an immutable reference to the [`CpuStorage`] associated with this [`CpuData`].
    pub fn storage(&self) -> &CpuStorage {
        // SAFETY:
        //
        // Since this [`CpuData`] exists and an immutable reference to it exists, it is safe to
        // provide an immutable reference to the [`CpuStorage`].
        unsafe { &*self.storage_ptr() }
    }

    /// Returns a mutable reference to the [`CpuStorage`] associated with this [`CpuData`].
    pub fn storage_mut(&mut self) -> &mut CpuStorage {
        // SAFETY:
        //
        // Since this [`CpuData`] exists and the mutable reference to it was passed to this
        // function, it is safe to provide a mutable reference to the [`CpuStorage`].
        unsafe { &mut *self.storage_ptr() }
    }
}

/// Various errors that can occur while allocating memory for a CPU.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuDataError {
    /// An error occurred while preparing the CPU's stack.
    Stack(ComponentError),
    /// An error occurred while preparing the CPU's [`CpuStorage`].
    Storage(ComponentError),
    /// An error occurred while preparing the CPU's stub code.
    StubCode(ComponentError),
    /// An error occurred while preparing the CPU's executable code.
    ExecutableCode(ComponentError),
}

impl fmt::Display for CpuDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stack(error) => write!(f, "error preparing stack: {error}"),
            Self::Storage(error) => write!(f, "error preparing storage: {error}"),
            Self::StubCode(error) => write!(f, "error preparing stub code: {error}"),
            Self::ExecutableCode(error) => write!(f, "error preparing executable code: {error}"),
        }
    }
}

/// The addresses and various other information about the layout of a block of code.
#[derive(Debug)]
pub struct CodeLayout {
    /// The [`FrameAllocation`] representing the physical memory in which the code resides.
    pub frame_allocation: FrameAllocation,

    /// The address of the `write` function.
    pub write: u64,
    /// The address of the `allocate_frames` function.
    pub allocate_frames: u64,
    /// The address of the `deallocate_frames` function.
    pub deallocate_frames: u64,
    /// The address of the `get_memory_map` function.
    pub get_memory_map: u64,
    /// The address of the `map` function.
    pub map: u64,
    /// The address of the `unmap` function.
    pub unmap: u64,
    /// The address of the `takeover` function.
    pub takeover: u64,
    /// The address of the `run_on_all_processors` function.
    pub run_on_all_processors: u64,

    /// Architecture-specific layout information.
    pub arch_code_layout: ArchCodeLayout,
}

/// 32-bit protocol table.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RevmProtocolTable32<ArchTable> {
    /// The [`Header`] that describes the entire table.
    pub header: Header,
    /// The architecture-independent portion of the table.
    pub generic_table: GenericTable32,
    /// The architecture-dependent portion of the table.
    pub arch_table: ArchTable,
}

impl<ArchTable> RevmProtocolTable32<ArchTable> {
    /// Replaces the current [`RevmProtocolTable32::arch_table`] with the provided value`.
    ///
    /// This also update the sizing and offset parameters in [`RevmProtocolTable32::header`].
    pub fn transpose<T>(self, arch_table: T) -> RevmProtocolTable32<T> {
        RevmProtocolTable32 {
            header: Header {
                version: Header::VERSION,
                last_major_version: Header::LAST_MAJOR_VERSION,
                length: usize_to_u64(mem::size_of::<RevmProtocolTable32<T>>()),
                generic_table_offset: usize_to_u64(mem::offset_of!(
                    RevmProtocolTable32<T>,
                    generic_table
                )),
                arch_table_offset: usize_to_u64(mem::offset_of!(
                    RevmProtocolTable32<T>,
                    arch_table
                )),
            },
            generic_table: self.generic_table,
            arch_table,
        }
    }
}

/// 64-bit protocol table.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RevmProtocolTable64<ArchTable> {
    /// The [`Header`] that describes the entire table.
    pub header: Header,
    /// The architecture-independent portion of the table.
    pub generic_table: GenericTable64,
    /// The architecture-dependent portion of the table.
    pub arch_table: ArchTable,
}

impl<ArchTable> RevmProtocolTable64<ArchTable> {
    /// Replaces the current [`RevmProtocolTable64::arch_table`] with the provided value`.
    ///
    /// This also update the sizing and offset parameters in [`RevmProtocolTable64::header`].
    pub fn transpose<T>(self, arch_table: T) -> RevmProtocolTable64<T> {
        RevmProtocolTable64 {
            header: Header {
                version: Header::VERSION,
                last_major_version: Header::LAST_MAJOR_VERSION,
                length: usize_to_u64(mem::size_of::<RevmProtocolTable64<T>>()),
                generic_table_offset: usize_to_u64(mem::offset_of!(
                    RevmProtocolTable64<T>,
                    generic_table
                )),
                arch_table_offset: usize_to_u64(mem::offset_of!(
                    RevmProtocolTable64<T>,
                    arch_table
                )),
            },
            generic_table: self.generic_table,
            arch_table,
        }
    }
}

/// Various errors that can occur while allocating and mapping a component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentError {
    /// An error occurred while allocating physical memory for the component.
    Allocation(OutOfMemory),
    /// An error occurred while finding a location for the allocated memory to be mapped into the
    /// executable's address space.
    FindRegion,
    /// An error occurred while mapping the allocated memory into the executable's address space.
    Map(MapError),
}

impl From<OutOfMemory> for ComponentError {
    fn from(error: OutOfMemory) -> Self {
        Self::Allocation(error)
    }
}

impl From<MapError> for ComponentError {
    fn from(error: MapError) -> Self {
        Self::Map(error)
    }
}

impl fmt::Display for ComponentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allocation(error) => write!(f, "error allocating physical memory: {error}"),
            Self::FindRegion => write!(f, "error locating free virtual memory"),
            Self::Map(error) => write!(
                f,
                "error mapping physical memory into virtual address space: {error}"
            ),
        }
    }
}

impl error::Error for ComponentError {}
