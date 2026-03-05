//! VMX-related functionality.

use core::{
    fmt,
    marker::PhantomData,
    mem::MaybeUninit,
    ptr,
    sync::atomic::{AtomicU32, Ordering},
};

use conversion::{u64_to_usize_strict, usize_to_u64};
use x86_common::{
    control::{Cr0, Cr4},
    cpuid::{cpuid_unchecked, supports_cpuid},
    msr::{read_msr, supports_msr, write_msr},
};

use crate::{
    arch::x86_common::virtualization::vmx::raw::*,
    memory::{
        page_frame_size,
        phys::{FrameAllocationError, allocate_frames, structs::FrameRange},
        virt::{MapError, Permissions, map, structs::PageRange},
    },
};

#[expect(clippy::missing_docs_in_private_items)]
mod raw;

/// Returns `true` if virtualization is supported.
pub fn supported() -> Option<VmxConfig> {
    if !supports_cpuid() {
        return None;
    }

    // SAFETY:
    //
    // `CPUID` is supported.
    let cpuid_result = unsafe { cpuid_unchecked(1, 0) };
    if (cpuid_result.ecx >> 5) & 0b1 != 0b1 {
        return None;
    }

    if !supports_msr() {
        return None;
    }

    // SAFETY:
    //
    // `RDMSR` is supported.
    let ia32_feature_control = unsafe { read_msr(IA32_FEATURE_CONTROL_MSR) };
    if ia32_feature_control & 0b1 == 0b1 {
        // Locked.
        if (ia32_feature_control >> 2) & 0b1 != 0b1 {
            return None;
        }
    }

    // SAFETY:
    //
    // `RDMSR` is supported.
    let basic = unsafe { read_msr(IA32_VMX_BASIC) };
    let basic = Basic(basic);

    let pin_based_ctls = if basic.true_controls_enabled() {
        // SAFETY:
        //
        // `RDMSR` is supported.
        unsafe { read_msr(IA32_VMX_TRUE_PINBASED_CTLS) }
    } else {
        // SAFETY:
        //
        // `RDMSR` is supported.
        unsafe { read_msr(IA32_VMX_PINBASED_CTLS) }
    };

    let proc_based_ctls = if basic.true_controls_enabled() {
        // SAFETY:
        //
        // `RDMSR` is supported.
        unsafe { read_msr(IA32_VMX_TRUE_PROCBASED_CTLS) }
    } else {
        // SAFETY:
        //
        // `RDMSR` is supported.
        unsafe { read_msr(IA32_VMX_PROCBASED_CTLS) }
    };

    let proc_based_ctls2 = if (proc_based_ctls >> 63) & 0b1 == 0b1 {
        // SAFETY:
        //
        // `RDMSR` is supported.
        unsafe { Some(read_msr(IA32_VMX_PROCBASED_CTLS2)) }
    } else {
        None
    };

    let proc_based_ctls3 = if (proc_based_ctls >> 49) & 0b1 == 0b1 {
        // SAFETY:
        //
        // `RDMSR` is supported.
        unsafe { Some(read_msr(IA32_VMX_PROCBASED_CTLS3)) }
    } else {
        None
    };

    let exit_ctls = if basic.true_controls_enabled() {
        // SAFETY:
        //
        // `RDMSR` is supported.
        unsafe { read_msr(IA32_VMX_TRUE_EXIT_CTLS) }
    } else {
        // SAFETY:
        //
        // `RDMSR` is supported.
        unsafe { read_msr(IA32_VMX_EXIT_CTLS) }
    };

    let exit_ctls2 = if (exit_ctls >> 63) & 0b1 == 0b1 {
        // SAFETY:
        //
        // `RDMSR` is supported.
        unsafe { Some(read_msr(IA32_VMX_EXIT_CTLS2)) }
    } else {
        None
    };

    let entry_ctls = if basic.true_controls_enabled() {
        // SAFETY:
        //
        // `RDMSR` is supported.
        unsafe { read_msr(IA32_VMX_TRUE_ENTRY_CTLS) }
    } else {
        // SAFETY:
        //
        // `RDMSR` is supported.
        unsafe { read_msr(IA32_VMX_ENTRY_CTLS) }
    };

    // SAFETY:
    //
    // `RDMSR` is supported.
    let misc = unsafe { read_msr(IA32_VMX_MISC) };

    // SAFETY:
    //
    // `RDMSR` is supported.
    let vmcs_enum = unsafe { read_msr(IA32_VMX_VMCS_ENUM) };

    // SAFETY:
    //
    // `RDMSR` is supported.
    let ept_vpid_cap = if (proc_based_ctls >> 63) & 0b1 == 0b1 {
        if proc_based_ctls2.is_some_and(|val| ((val >> 33) | (val >> 37)) & 0b1 == 0b1) {
            // SAFETY:
            //
            // `RDMSR` is supported.
            unsafe { Some(read_msr(IA32_VMX_EPT_VPID_CAP)) }
        } else {
            None
        }
    } else {
        None
    };

    let vmfunc = if (proc_based_ctls >> 63) & 0b1 == 0b1 {
        if proc_based_ctls2.is_some_and(|val| (val >> 45) & 0b1 == 0b1) {
            // SAFETY:
            //
            // `RDMSR` is supported.
            unsafe { Some(read_msr(IA32_VMX_VMFUNC)) }
        } else {
            None
        }
    } else {
        None
    };

    // SAFETY:
    //
    // `RDMSR` is supported.
    let cr0_fixed_0 = unsafe { read_msr(IA32_VMX_CR0_FIXED0) };
    // SAFETY:
    //
    // `RDMSR` is supported.
    let cr0_fixed_1 = unsafe { read_msr(IA32_VMX_CR0_FIXED1) };
    // SAFETY:
    //
    // `RDMSR` is supported.
    let cr4_fixed_0 = unsafe { read_msr(IA32_VMX_CR4_FIXED0) };
    // SAFETY:
    //
    // `RDMSR` is supported.
    let cr4_fixed_1 = unsafe { read_msr(IA32_VMX_CR4_FIXED1) };

    let vmx_config = VmxConfig {
        basic,
        pin_based_ctls,
        proc_based_ctls,
        proc_based_ctls2,
        proc_based_ctls3,
        exit_ctls,
        exit_ctls2,
        entry_ctls,
        misc,
        vmcs_enum,
        ept_vpid_cap,
        vmfunc,
        cr0_fixed_0,
        cr0_fixed_1,
        cr4_fixed_0,
        cr4_fixed_1,
    };

    Some(vmx_config)
}

#[derive(Clone, Copy, Debug)]
#[expect(clippy::missing_docs_in_private_items)]
pub struct VmxConfig {
    basic: Basic,
    pin_based_ctls: u64,
    proc_based_ctls: u64,
    proc_based_ctls2: Option<u64>,
    proc_based_ctls3: Option<u64>,
    exit_ctls: u64,
    exit_ctls2: Option<u64>,
    entry_ctls: u64,
    misc: u64,
    vmcs_enum: u64,
    ept_vpid_cap: Option<u64>,
    vmfunc: Option<u64>,
    cr0_fixed_0: u64,
    cr0_fixed_1: u64,
    cr4_fixed_0: u64,
    cr4_fixed_1: u64,
}

impl VmxConfig {
    /// Returns the [`Cr0`] bits that must be off when VMX is enabled.
    pub const fn cr0_off(&self) -> Cr0 {
        Cr0::from_bits(!self.cr0_fixed_1)
    }

    /// Returns the [`Cr0`] bits that can be either off or on when VMX is enabled.
    pub const fn cr0_flexible(&self) -> Cr0 {
        Cr0::from_bits(self.cr0_fixed_1 & !self.cr0_fixed_0)
    }

    /// Returns the [`Cr0`] bits that must be on when VMX is enabled.
    pub const fn cr0_on(&self) -> Cr0 {
        Cr0::from_bits(self.cr0_fixed_0)
    }

    /// Returns the [`Cr4`] bits that must be off when VMX is enabled.
    pub const fn cr4_off(&self) -> Cr0 {
        Cr0::from_bits(!self.cr4_fixed_1)
    }

    /// Returns the [`Cr4`] bits that can be either off or on when VMX is enabled.
    pub const fn cr4_flexible(&self) -> Cr0 {
        Cr0::from_bits(self.cr4_fixed_1 & !self.cr4_fixed_0)
    }

    /// Returns the [`Cr4`] bits that must be on when VMX is enabled.
    pub const fn cr4_on(&self) -> Cr0 {
        Cr0::from_bits(self.cr4_fixed_0)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct Basic(u64);

impl Basic {
    /// Returns the VMCS revision identifier (bits 0–30)
    pub fn revision(&self) -> u32 {
        (self.0 & 0x7FFF_FFFF) as u32
    }

    /// Returns the region size in bytes (bits 32–44)
    pub fn region_size(&self) -> u16 {
        ((self.0 >> 32) & 0x1FFF) as u16
    }

    /// Returns whether the processor requires the VMCS region to be 32-bit (bit 48)
    pub fn fixed_32_bit(&self) -> bool {
        (self.0 >> 48) & 0b1 != 0
    }

    /// Returns whether dual-monitor treatment of interrupts is supported (bit 49)
    pub fn dual_monitor(&self) -> bool {
        (self.0 >> 49) & 0b1 != 0
    }

    /// Returns the memory type for the VMCS region (bits 50–53)
    pub fn memory_type(&self) -> u8 {
        ((self.0 >> 50) & 0b1111) as u8
    }

    /// Returns whether the processor reports information in the VM-exit instruction-information
    /// field on VM-exits due to the execution of `INS` and `OUTS` instructions.
    pub fn vm_exit_string(&self) -> bool {
        (self.0 >> 54) & 0b1 != 0
    }

    /// Returns whether “true controls” are available (bit 55)
    pub fn true_controls_enabled(&self) -> bool {
        (self.0 >> 55) & 0b1 != 0
    }

    /// Returns whether software can deliver hardware exceptions with or without an error code
    /// regardless of vector.
    pub fn hardware_exception_delivery(&self) -> bool {
        (self.0 >> 56) & 0b1 != 0
    }
}

impl fmt::Debug for Basic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("Basic");

        debug_struct.field("raw", &self.0);
        debug_struct.field("revision", &self.revision());
        debug_struct.field("region_size", &self.region_size());
        debug_struct.field("fixed_32_bit", &self.fixed_32_bit());
        debug_struct.field("dual_monitor", &self.dual_monitor());
        debug_struct.field("memory_type", &self.memory_type());
        debug_struct.field("vm_exit_string", &self.vm_exit_string());
        debug_struct.field("true_controls_enabled", &self.true_controls_enabled());
        debug_struct.field(
            "hardware_exception_delivery",
            &self.hardware_exception_delivery(),
        );

        debug_struct.finish()
    }
}

pub fn enable(config: VmxConfig) -> Result<(), VmxEnableError> {
    // SAFETY:
    //
    // `RDMSR` is supported.
    let ia32_feature_control = unsafe { read_msr(IA32_FEATURE_CONTROL_MSR) };
    if ia32_feature_control & 0b1 == 0b1 {
        if ia32_feature_control & 0b100 == 0 {
            // IA32_FEATURE_CONTROL_MSR has been locked into a invalid state.
            todo!("implement virtualization enable failure");
        }

        // IA32_FEATURE_CONTROL_MSR has already been locked into a valid state.
    } else {
        crate::trace!("IA32_FEATURE_CONTROL_MSR: Enabling VMX and locking");
        // SAFETY:
        //
        // `WRMSR` is supported.
        unsafe { write_msr(IA32_FEATURE_CONTROL_MSR, ia32_feature_control | 0b101) }
    }

    // SAFETY:
    //
    // `revm` always operates in ring 0 and thus accessing the CR0 register is always safe.
    let cr0_before = unsafe { Cr0::get() };
    // SAFETY:
    //
    // `revm` always operates in ring 0 and thus accessing the CR4 register is always safe.
    let cr4_before = unsafe { Cr4::get() };

    // Keep flexible bits and enable forced on bits.
    let cr0_after_bits =
        (config.cr0_flexible().to_bits() & cr0_before.to_bits()) | config.cr0_on().to_bits();
    let cr0_after = Cr0::from_bits(cr0_after_bits);

    // Keep flexible bits, enable forced on bits, and explicitly enable VMXE.
    let cr4_after_bits =
        (config.cr4_flexible().to_bits() & cr4_before.to_bits()) | config.cr4_on().to_bits();
    let cr4_after = Cr4::from_bits(cr4_after_bits).set_vmxe(true);

    let cr0_forced_off = config.cr0_off();
    let cr0_flexible = config.cr0_flexible();
    let cr0_forced_on = config.cr0_on();
    let cr0_changed = Cr0::from_bits(cr0_before.to_bits() ^ cr0_after.to_bits());

    crate::debug!("CR0 Forced Off: {cr0_forced_off}");
    crate::debug!("CR0 Flexible  : {cr0_flexible}");
    crate::debug!("CR0 Forced On : {cr0_forced_on}");
    crate::debug!("CR0 Before    : {cr0_before}");
    crate::debug!("CR0 After     : {cr0_after}");
    crate::debug!("CR0 Changed   : {cr0_changed}");

    let cr4_forced_off = config.cr4_off();
    let cr4_flexible = config.cr4_flexible();
    let cr4_forced_on = config.cr4_on();
    let cr4_changed = Cr4::from_bits(cr4_before.to_bits() ^ cr4_after.to_bits());

    crate::debug!("CR4 Forced Off: {cr4_forced_off}");
    crate::debug!("CR4 Flexible  : {cr4_flexible}");
    crate::debug!("CR4 Forced On : {cr4_forced_on}");
    crate::debug!("CR4 Before    : {cr4_before}");
    crate::debug!("CR4 After     : {cr4_after}");
    crate::debug!("CR4 Changed   : {cr4_changed}");

    // SAFETY:
    //
    // `revm` always operates in ring 0 and thus accessing the CR0 register is always safe. The CR0
    // value has been checked with VMX requirements and is valid.
    unsafe { cr0_after.set() }
    crate::trace!("CR0 successfully set");

    // SAFETY:
    //
    // `revm` always operates in ring 0 and thus accessing the CR4 register is always safe. The CR4
    // value has been checked with VMX requirements and is valid.
    unsafe { cr4_after.set() }
    crate::trace!("CR4 successfully set");

    let vmxon_frame_range =
        allocate_frames(usize_to_u64(4096usize.div_ceil(page_frame_size())), 4096)?;
    let vmxon_page_range = map(vmxon_frame_range, Permissions::ReadWrite)?;
    let vmxon_ptr = ptr::with_exposed_provenance_mut::<MaybeUninit<AtomicU32>>(
        vmxon_page_range.start_address().value(),
    );

    // SAFETY:
    //
    // MaybeUninit regions do not require initialization and the frame and memory allocation
    // primitives have provided a 4096-byte physical and virtual region to manipulate.
    let vmxon = unsafe { &mut *vmxon_ptr };
    let vmxon = vmxon.write(AtomicU32::new(config.basic.revision()));
    let vmxon = VmxOn {
        frame_range: vmxon_frame_range,
        page_range: vmxon_page_range,
        vmxon,
        phantom: PhantomData,
    };

    let vmcs_frame_range =
        allocate_frames(usize_to_u64(4096usize.div_ceil(page_frame_size())), 4096)?;
    let vmcs_page_range = map(vmcs_frame_range, Permissions::ReadWrite)?;
    let vmcs_ptr = ptr::with_exposed_provenance_mut::<MaybeUninit<[AtomicU32; 2]>>(
        vmcs_page_range.start_address().value(),
    );

    // SAFETY:
    //
    // MaybeUninit regions do not require initialization and the frame and memory allocation
    // primitives have provided a 4096-byte physical and virtual region to manipulate.
    let vmcs = unsafe { &mut *vmcs_ptr };
    let vmcs = vmcs.write([AtomicU32::new(config.basic.revision()), AtomicU32::new(0)]);
    let vmcs = Vmcs {
        frame_range: vmcs_frame_range,
        page_range: vmcs_page_range,
        vmcs,
        phantom: PhantomData,
    };

    crate::trace!("Executing VMXON");

    let vmxon_ptr = u64_to_usize_strict(vmxon.frame_range.start_address().value());
    // SAFETY:
    //
    // VMXON region has been properly initialized.
    unsafe { core::arch::asm!("vmxon [{}]", in(reg) &vmxon_ptr) }

    let vmcs_ptr = u64_to_usize_strict(vmcs.frame_range.start_address().value());
    // SAFETY:
    //
    // VMXON region has been properly initialized.
    unsafe {
        core::arch::asm!(
            "vmptrld [{vm_phys_addr}]",
            "vmclear [{vm_phys_addr}]",
            "vmptrld [{vm_phys_addr}]",
            vm_phys_addr = in(reg) vmcs_ptr,
        )
    }

    loop {}

    todo!("{config:#x?}")
}

#[derive(Debug)]
pub enum VmxEnableError {}

impl From<FrameAllocationError> for VmxEnableError {
    fn from(error: FrameAllocationError) -> Self {
        todo!("{error}")
    }
}

impl From<MapError> for VmxEnableError {
    fn from(error: MapError) -> Self {
        todo!("{error}")
    }
}

pub struct VmxOn {
    frame_range: FrameRange,
    page_range: PageRange,
    vmxon: &'static mut AtomicU32,
    // Force !Send and !Sync.
    phantom: PhantomData<*mut ()>,
}

pub struct Vmcs {
    frame_range: FrameRange,
    page_range: PageRange,
    vmcs: &'static mut [AtomicU32; 2],
    // Force !Send and !Sync.
    phantom: PhantomData<*mut ()>,
}
