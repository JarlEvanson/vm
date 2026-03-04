//! Implementation of cross-address space switching functionality for `x86_32`.

use core::mem::offset_of;

use conversion::usize_to_u64;
use memory::{
    address::{AddressChunk, AddressChunkRange, PhysicalAddressRange},
    phys::PhysicalMemorySpace,
    translation::{MapFlags, TranslationScheme},
};

use crate::{
    arch::{
        generic::switch::{
            func::{
                ALLOCATE_FRAMES_FUNC_ID, DEALLOCATE_FRAMES_FUNC_ID, ENTER_FUNC_ID,
                EXEC_ON_PROCESSOR_FUNC_ID, GET_MEMORY_MAP_FUNC_ID, MAP_FUNC_ID, RETURN_FUNC_ID,
                RUN_ON_ALL_PROCESSORS_FUNC_ID, TAKEOVER_FUNC_ID, UNMAP_FUNC_ID, WRITE_FUNC_ID,
            },
            setup::{CodeLayout, ComponentError},
        },
        paging::ArchScheme,
        switch::{ArchCodeLayout, CpuStorage, arch_policy},
        x86_common::switch::{CallStorage, ModeStorage},
    },
    platform::{
        StubPhysicalMemory, allocate_frames_aligned, frame_size, map_identity, write_bytes_at,
    },
};

/// Allocates and maps the switching code.
pub fn allocate_code(
    scheme: &mut ArchScheme,
    storage_pointer: u64,
    own_mode_storage_pointer: u64,
    other_mode_storage_pointer: u64,
) -> Result<CodeLayout, ComponentError> {
    let code_start_ptr = &raw const X86_32_CODE_START;
    let code_end_ptr = &raw const X86_32_CODE_END;
    let code_size = code_end_ptr.addr().strict_sub(code_start_ptr.addr());
    let code_size_u64 = usize_to_u64(code_size);

    let frame_allocation = allocate_frames_aligned(
        code_size_u64.div_ceil(frame_size()),
        scheme.chunk_size(),
        arch_policy(),
    )?;

    // Map code region into the two address spaces.
    let _ = map_identity(PhysicalAddressRange::new(
        frame_allocation.range().start().start_address(frame_size()),
        code_size_u64,
    ));

    let chunk_range = AddressChunkRange::new(
        AddressChunk::containing_address(
            frame_allocation
                .range()
                .start()
                .start_address(frame_size())
                .to_address(),
            scheme.chunk_size(),
        ),
        code_size_u64.div_ceil(scheme.chunk_size()),
    );

    // SAFETY:
    //
    // This [`ArchScheme`] is not actively in use.
    unsafe {
        scheme.map(
            chunk_range,
            chunk_range,
            MapFlags::READ | MapFlags::WRITE | MapFlags::EXEC,
        )?;
    }

    // Fill the code region with the provided code.

    // SAFETY:
    //
    // The allocated region is part of `revm-stub` and is not writable and thus it is safe to
    // create an immutable slice.
    let code_bytes = unsafe { core::slice::from_raw_parts(code_start_ptr, code_size) };
    write_bytes_at(
        frame_allocation.range().start().start_address(frame_size()),
        code_bytes,
    );

    // Fill the `storage_pointer` and `mode_storage_ptr` in the assembly code.

    // SAFETY:
    //
    // The frame allocation was large enough for these ranges to be under the exclusive control of
    // this program.
    #[expect(clippy::multiple_unsafe_ops_per_block)]
    unsafe {
        StubPhysicalMemory.write_u64_le(
            frame_allocation.range().start().start_address(frame_size()),
            storage_pointer,
        );
        StubPhysicalMemory.write_u64_le(
            frame_allocation
                .range()
                .start()
                .start_address(frame_size())
                .strict_add(8),
            own_mode_storage_pointer,
        );
        StubPhysicalMemory.write_u64_le(
            frame_allocation
                .range()
                .start()
                .start_address(frame_size())
                .strict_add(16),
            other_mode_storage_pointer,
        );
    }

    let offset = usize_to_u64(code_start_ptr.addr()).wrapping_sub(
        frame_allocation
            .range()
            .start()
            .start_address(frame_size())
            .value(),
    );

    let enter_mode_ptr = &raw const X86_32_CODE_ENTER_MODE;
    let enter_mode = usize_to_u64(enter_mode_ptr.addr()).wrapping_sub(offset);

    let call_handler_ptr = &raw const X86_32_CODE_CALL_HANDLER;
    let call_handler = usize_to_u64(call_handler_ptr.addr()).wrapping_sub(offset);

    let call_ptr = &raw const X86_32_CODE_CALL;
    let call = usize_to_u64(call_ptr.addr()).wrapping_sub(offset);

    let write_ptr = &raw const X86_32_CODE_WRITE;
    let write = usize_to_u64(write_ptr.addr()).wrapping_sub(offset);

    let allocate_frames_ptr = &raw const X86_32_CODE_ALLOCATE_FRAMES;
    let allocate_frames = usize_to_u64(allocate_frames_ptr.addr()).wrapping_sub(offset);

    let deallocate_frames_ptr = &raw const X86_32_CODE_DEALLOCATE_FRAMES;
    let deallocate_frames = usize_to_u64(deallocate_frames_ptr.addr()).wrapping_sub(offset);

    let get_memory_map_ptr = &raw const X86_32_CODE_GET_MEMORY_MAP;
    let get_memory_map = usize_to_u64(get_memory_map_ptr.addr()).wrapping_sub(offset);

    let map_ptr = &raw const X86_32_CODE_MAP;
    let map = usize_to_u64(map_ptr.addr()).wrapping_sub(offset);

    let unmap_ptr = &raw const X86_32_CODE_UNMAP;
    let unmap = usize_to_u64(unmap_ptr.addr()).wrapping_sub(offset);

    let takeover_ptr = &raw const X86_32_CODE_TAKEOVER;
    let takeover = usize_to_u64(takeover_ptr.addr()).wrapping_sub(offset);

    let run_on_all_processors_ptr = &raw const X86_32_CODE_RUN_ON_ALL_PROCESSORS;
    let run_on_all_processors = usize_to_u64(run_on_all_processors_ptr.addr()).wrapping_sub(offset);

    let code_layout = CodeLayout {
        frame_allocation,

        write,
        allocate_frames,
        deallocate_frames,
        get_memory_map,
        map,
        unmap,
        takeover,
        run_on_all_processors,

        arch_code_layout: ArchCodeLayout {
            enter_mode,
            call_handler,
            call,
        },
    };

    Ok(code_layout)
}

unsafe extern "C" {
    static X86_32_CODE_START: u8;

    static X86_32_CODE_ENTER_MODE: u8;

    static X86_32_CODE_CALL_HANDLER: u8;
    static X86_32_CODE_CALL: u8;

    static X86_32_CODE_WRITE: u8;
    static X86_32_CODE_ALLOCATE_FRAMES: u8;
    static X86_32_CODE_DEALLOCATE_FRAMES: u8;
    static X86_32_CODE_GET_MEMORY_MAP: u8;
    static X86_32_CODE_MAP: u8;
    static X86_32_CODE_UNMAP: u8;
    static X86_32_CODE_TAKEOVER: u8;
    static X86_32_CODE_RUN_ON_ALL_PROCESSORS: u8;

    static X86_32_CODE_END: u8;
}

core::arch::global_asm! {
    ".code32",

    ".global X86_32_CODE_START",
    "X86_32_CODE_START:",

    "5:",

    // Allocate space for pointer to the [`Storage`] that this stub will use.
    "storage_pointer:",
    ".8byte 0",

    "4:",
    // Allocate space for pointer to the [`ModeStorage`] that this stub will use.
    "own_mode_storage_pointer:",
    ".8byte 0",

    "3:",
    // Allocate space for pointer to the [`ModeStorage`] that the other entity will use.
    "other_mode_storage_pointer:",
    ".8byte 0",

    ".equ own_mode_x86_32_offset, 4b - 5b",
    ".equ other_mode_x86_32_offset, 3b - 5b",

    "get_base:",

    "call 5f",
    "5:",

    ".equ call_offset, 5b - X86_32_CODE_START",

    "pop ecx",
    "sub ecx, offset call_offset",

    "ret",

    // Transitions to the other address space.
    "transition:",

    "call get_base",

    "mov ecx, [ecx + own_mode_x86_32_offset]",
    "mov [ecx + {MODE_STORAGE_RAX}], eax",

    "mov eax, ecx",
    "mov [eax + {MODE_STORAGE_RBX}], ebx",
    "mov [eax + {MODE_STORAGE_RCX}], ecx",
    "mov [eax + {MODE_STORAGE_RDX}], edx",

    "mov [eax + {MODE_STORAGE_RSI}], esi",
    "mov [eax + {MODE_STORAGE_RDI}], edi",
    "mov [eax + {MODE_STORAGE_RSP}], esp",
    "mov [eax + {MODE_STORAGE_RBP}], ebp",

    "mov bx, cs",
    "mov [eax + {MODE_STORAGE_CS}], bx",
    "mov bx, ds",
    "mov [eax + {MODE_STORAGE_DS}], bx",
    "mov bx, es",
    "mov [eax + {MODE_STORAGE_ES}], bx",
    "mov bx, fs",
    "mov [eax + {MODE_STORAGE_FS}], bx",
    "mov bx, gs",
    "mov [eax + {MODE_STORAGE_GS}], bx",
    "mov bx, ss",
    "mov [eax + {MODE_STORAGE_SS}], bx",

    "mov ebx, cr0",
    "mov [eax + {MODE_STORAGE_CR0}], ebx",
    "mov ebx, cr3",
    "mov [eax + {MODE_STORAGE_CR3}], ebx",
    "mov ebx, cr4",
    "mov [eax + {MODE_STORAGE_CR4}], ebx",

    "sgdt [eax + {MODE_STORAGE_GDTR}]",
    "sidt [eax + {MODE_STORAGE_IDTR}]",

    "call get_base",
    "mov ebp, [ecx + own_mode_x86_32_offset]",
    "mov ecx, [ecx]",
    "mov bl, [ecx + {STORAGE_CHANGE_EFER}]",
    "cmp bl, 0",
    "je 5f",

    "mov ecx, 0xC0000080",
    "rdmsr",

    "mov [ebp + {MODE_STORAGE_EFER}], eax",
    "mov [ebp + {MODE_STORAGE_EFER} + 4], edx",

    "5:",

    "call get_base",
    "mov esp, [ecx + other_mode_x86_32_offset]",
    "mov ebp, [ecx + own_mode_x86_32_offset]",
    "mov edi, [ecx]",

    // Get current `CR0` value.
    "mov ecx, cr0",

    // Disable paging bit.
    "mov edx, ~(1 << 31)",
    "and ecx, edx",

    // Set current `CR0` value.
    "mov cr0, ecx",

    "mov ecx, [esp + {MODE_STORAGE_HANDLE_ENTER_MODE}]",
    "jmp ecx",

    ".global X86_32_CODE_ENTER_MODE",
    "X86_32_CODE_ENTER_MODE:",

    "mov al, [edi + {STORAGE_CHANGE_EFER}]",
    "cmp al, 0",
    "je 5f",

    "mov eax, [esp + {MODE_STORAGE_EFER}]",
    "mov edx, [esp + {MODE_STORAGE_EFER} + 4]",
    "mov ecx, 0xC0000080",
    "wrmsr",

    "5:",

    // Load control registers.
    "mov ecx, [esp + {MODE_STORAGE_CR3}]",
    "mov cr3, ecx",

    "mov ecx, [esp + {MODE_STORAGE_CR4}]",
    "mov cr4, ecx",

    "mov ecx, [esp + {MODE_STORAGE_CR0}]",
    "mov cr0, ecx",

    // Load GDT/IDT.
    "lgdt [esp + {MODE_STORAGE_GDTR}]",
    "lidt [esp + {MODE_STORAGE_IDTR}]",

    "mov ax, [esp + {MODE_STORAGE_DS}]",
    "mov ds, ax",
    "mov ax, [esp + {MODE_STORAGE_ES}]",
    "mov es, ax",
    "mov ax, [esp + {MODE_STORAGE_FS}]",
    "mov fs, ax",
    "mov ax, [esp + {MODE_STORAGE_GS}]",
    "mov gs, ax",
    "mov ax, [esp + {MODE_STORAGE_SS}]",
    "mov ss, ax",

    // Don't load `eax` or `ecx` since they are scratch registers
    // and `eax` holds the address of our mode storage.
    "mov eax, esp",

    "mov ebx, [eax + {MODE_STORAGE_RBX}]",
    "mov ecx, [eax + {MODE_STORAGE_RCX}]",
    "mov edx, [eax + {MODE_STORAGE_RDX}]",

    "mov esi, [eax + {MODE_STORAGE_RSI}]",
    "mov edi, [eax + {MODE_STORAGE_RDI}]",
    "mov esp, [eax + {MODE_STORAGE_RSP}]",
    "mov ebp, [eax + {MODE_STORAGE_RBP}]",

    "sub esp, 8",

    "mov cx, [eax + {MODE_STORAGE_CS}]",
    "mov [esp + 4], cx",

    "call 5f",
    "5:",

    ".equ ljmp_offset, 4f - 5b",

    "pop ecx",
    "add ecx, offset ljmp_offset",
    "mov [esp], ecx",

    "ljmp [esp]",
    "4:",

    "add esp, 8",

    // Load [`Storage`] address.
    "call get_base",
    "mov ecx, [ecx]",

    // Load [`CallStorage`] address.
    "lea edx, [ecx + {CALL_STORAGE_OFFSET}]",

    // Determine whether to return or to handle the call.
    "mov cx, [edx + {CALL_STORAGE_FUNC_ID_OFFSET}]",
    "cmp cx, 0",
    "jne 5f",

    // Return.
    "mov eax, [edx + {CALL_STORAGE_RET_OFFSET}]",
    "mov edx, [edx + {CALL_STORAGE_RET_OFFSET} + 4]",
    "ret",

    // Handle a call.
    "5:",

    "call get_base",
    "mov ecx, [ecx + own_mode_x86_32_offset]",
    "mov eax, [ecx + {MODE_STORAGE_HANDLE_CALL_HANDLER}]",

    "call eax",

    // Load [`Storage`] address.
    "call get_base",
    "mov ecx, [ecx]",

    // Load [`CallStorage`] address.
    "lea ecx, [ecx + {CALL_STORAGE_OFFSET}]",

    "mov word ptr [ecx + {CALL_STORAGE_FUNC_ID_OFFSET}], {RETURN_FUNC_ID}",
    "mov byte ptr [ecx + {CALL_STORAGE_ARG_COUNT_OFFSET}], 1",

    "mov [ecx + {CALL_STORAGE_RET_OFFSET}], eax",
    "mov [ecx + {CALL_STORAGE_RET_OFFSET} + 4], edx",

    "jmp transition",
    "5:", "jmp 5b",

    ".global X86_32_CODE_CALL_HANDLER",
    "X86_32_CODE_CALL_HANDLER:",

    // Load [`Storage`] address.
    "call get_base",
    "mov ecx, [ecx]",

    // Load [`CallStorage`] address.
    "mov ax, [ecx + {CALL_STORAGE_OFFSET} + {CALL_STORAGE_FUNC_ID_OFFSET}]",
    "cmp ax, {ENTER_FUNC_ID}",
    "jne 5f",

    // When [`ENTER_FUNC_ID`] is called, the first argument is the entry point and the second is
    // the first argument to the entry point.
    "call get_base",
    "mov ecx, [ecx]",
    "mov eax, [ecx + {CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_1_OFFSET}]",

    // Setup stack-based arguments.
    "push eax",

    "mov eax, [ecx + {CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_0_OFFSET}]",
    "call eax",

    // Remove stack-based arguments.
    "pop ecx",

    "ret",

    "5:",
    "cmp ax, {EXEC_ON_PROCESSOR_FUNC_ID}",
    "jne 5f",

    // When [`EXEC_ON_PROCESSOR_FUNC_ID`] is called, the first argument is the function to run, the
    // second is the CPU ID, and the third argument is the user-defined argument to the function.
    "call get_base",
    "mov ecx, [ecx]",
    "mov eax, [ecx + {CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_2_OFFSET}]",

    "push eax",

    "mov eax, [ecx + {CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_1_OFFSET} + 4]",
    "push eax",
    "mov eax, [ecx + {CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_1_OFFSET}]",
    "push eax",

    "mov eax, [ecx + {CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_0_OFFSET}]",
    "call eax",

    "add esp, 12",
    "ret",

    "5:",
    "jmp 5b",

    ".global X86_32_CODE_CALL",
    "X86_32_CODE_CALL:",

    "call transition",

    "ret",

    ".global X86_32_CODE_WRITE",
    "X86_32_CODE_WRITE:",

    // Load [`Storage`] address.
    "call get_base",
    "mov ecx, [ecx]",

    // Load [`CallStorage`] address.
    "lea ecx, [ecx + {CALL_STORAGE_OFFSET}]",

    "mov eax, {WRITE_FUNC_ID}",
    "mov [ecx + {CALL_STORAGE_FUNC_ID_OFFSET}], ax",

    "mov eax, 2",
    "mov [ecx + {CALL_STORAGE_ARG_COUNT_OFFSET}], al",

    "mov eax, [esp + 4]",
    "mov [ecx + {CALL_STORAGE_ARG_0_OFFSET}], eax",
    "xor eax, eax",
    "mov [ecx + {CALL_STORAGE_ARG_0_OFFSET} + 4], eax",

    "mov eax, [esp + 8]",
    "mov [ecx + {CALL_STORAGE_ARG_1_OFFSET}], eax",
    "xor eax, eax",
    "mov [ecx + {CALL_STORAGE_ARG_1_OFFSET} + 4], eax",

    "call transition",

    "ret",

    ".global X86_32_CODE_ALLOCATE_FRAMES",
    "X86_32_CODE_ALLOCATE_FRAMES:",

    // Load [`Storage`] address.
    "call get_base",
    "mov ecx, [ecx]",

    // Load [`CallStorage`] address.
    "lea ecx, [ecx + {CALL_STORAGE_OFFSET}]",

    "mov eax, {ALLOCATE_FRAMES_FUNC_ID}",
    "mov [ecx + {CALL_STORAGE_FUNC_ID_OFFSET}], ax",

    "mov eax, 4",
    "mov [ecx + {CALL_STORAGE_ARG_COUNT_OFFSET}], al",

    "mov eax, [esp + 4]",
    "mov [ecx + {CALL_STORAGE_ARG_0_OFFSET}], eax",
    "mov eax, [esp + 8]",
    "mov [ecx + {CALL_STORAGE_ARG_0_OFFSET} + 4], eax",

    "mov eax, [esp + 12]",
    "mov [ecx + {CALL_STORAGE_ARG_1_OFFSET}], eax",
    "mov eax, [esp + 16]",
    "mov [ecx + {CALL_STORAGE_ARG_1_OFFSET} + 4], eax",

    "mov eax, [esp + 20]",
    "mov [ecx + {CALL_STORAGE_ARG_2_OFFSET}], eax",
    "mov eax, [esp + 24]",
    "mov [ecx + {CALL_STORAGE_ARG_2_OFFSET} + 4], eax",

    "mov eax, [esp + 28]",
    "mov [ecx + {CALL_STORAGE_ARG_3_OFFSET}], eax",
    "xor eax, eax",
    "mov [ecx + {CALL_STORAGE_ARG_3_OFFSET} + 4], eax",

    "call transition",

    "ret",

    ".global X86_32_CODE_DEALLOCATE_FRAMES",
    "X86_32_CODE_DEALLOCATE_FRAMES:",

    // Load [`Storage`] address.
    "call get_base",
    "mov ecx, [ecx]",

    // Load [`CallStorage`] address.
    "lea ecx, [ecx + {CALL_STORAGE_OFFSET}]",

    "mov eax, {DEALLOCATE_FRAMES_FUNC_ID}",
    "mov [ecx + {CALL_STORAGE_FUNC_ID_OFFSET}], ax",

    "mov eax, 2",
    "mov [ecx + {CALL_STORAGE_ARG_COUNT_OFFSET}], al",

    "mov eax, [esp + 4]",
    "mov [ecx + {CALL_STORAGE_ARG_0_OFFSET}], eax",
    "mov eax, [esp + 8]",
    "mov [ecx + {CALL_STORAGE_ARG_0_OFFSET} + 4], eax",

    "mov eax, [esp + 12]",
    "mov [ecx + {CALL_STORAGE_ARG_1_OFFSET}], eax",
    "mov eax, [esp + 16]",
    "mov [ecx + {CALL_STORAGE_ARG_1_OFFSET} + 4], eax",

    "call transition",

    "ret",

    ".global X86_32_CODE_GET_MEMORY_MAP",
    "X86_32_CODE_GET_MEMORY_MAP:",

    // Load [`Storage`] address.
    "call get_base",
    "mov ecx, [ecx]",

    // Load [`CallStorage`] address.
    "lea ecx, [ecx + {CALL_STORAGE_OFFSET}]",

    "mov eax, {GET_MEMORY_MAP_FUNC_ID}",
    "mov [ecx + {CALL_STORAGE_FUNC_ID_OFFSET}], ax",

    "mov eax, 5",
    "mov [ecx + {CALL_STORAGE_ARG_COUNT_OFFSET}], al",

    "mov eax, [esp + 4]",
    "mov [ecx + {CALL_STORAGE_ARG_0_OFFSET}], eax",
    "xor eax, eax",
    "mov [ecx + {CALL_STORAGE_ARG_0_OFFSET} + 4], eax",

    "mov eax, [esp + 8]",
    "mov [ecx + {CALL_STORAGE_ARG_1_OFFSET}], eax",
    "xor eax, eax",
    "mov [ecx + {CALL_STORAGE_ARG_1_OFFSET} + 4], eax",

    "mov eax, [esp + 12]",
    "mov [ecx + {CALL_STORAGE_ARG_2_OFFSET}], eax",
    "xor eax, eax",
    "mov [ecx + {CALL_STORAGE_ARG_2_OFFSET} + 4], eax",

    "mov eax, [esp + 16]",
    "mov [ecx + {CALL_STORAGE_ARG_3_OFFSET}], eax",
    "xor eax, eax",
    "mov [ecx + {CALL_STORAGE_ARG_3_OFFSET} + 4], eax",

    "mov eax, [esp + 20]",
    "mov [ecx + {CALL_STORAGE_ARG_4_OFFSET}], eax",
    "xor eax, eax",
    "mov [ecx + {CALL_STORAGE_ARG_4_OFFSET} + 4], eax",

    "call transition",

    "ret",

    ".global X86_32_CODE_MAP",
    "X86_32_CODE_MAP:",

    // Load [`Storage`] address.
    "call get_base",
    "mov ecx, [ecx]",

    // Load [`CallStorage`] address.
    "lea ecx, [ecx + {CALL_STORAGE_OFFSET}]",

    "mov eax, {MAP_FUNC_ID}",
    "mov [ecx + {CALL_STORAGE_FUNC_ID_OFFSET}], ax",

    "mov eax, 4",
    "mov [ecx + {CALL_STORAGE_ARG_COUNT_OFFSET}], al",

    "mov eax, [esp + 4]",
    "mov [ecx + {CALL_STORAGE_ARG_0_OFFSET}], eax",
    "mov eax, [esp + 8]",
    "mov [ecx + {CALL_STORAGE_ARG_0_OFFSET} + 4], eax",

    "mov eax, [esp + 12]",
    "mov [ecx + {CALL_STORAGE_ARG_1_OFFSET}], eax",
    "xor eax, eax",
    "mov [ecx + {CALL_STORAGE_ARG_1_OFFSET} + 4], eax",

    "mov eax, [esp + 16]",
    "mov [ecx + {CALL_STORAGE_ARG_2_OFFSET}], eax",
    "xor eax, eax",
    "mov [ecx + {CALL_STORAGE_ARG_2_OFFSET} + 4], eax",

    "mov eax, [esp + 20]",
    "mov [ecx + {CALL_STORAGE_ARG_3_OFFSET}], eax",
    "mov eax, [esp + 24]",
    "mov [ecx + {CALL_STORAGE_ARG_3_OFFSET} + 4], eax",

    "call transition",

    "ret",

    ".global X86_32_CODE_UNMAP",
    "X86_32_CODE_UNMAP:",

    // Load [`Storage`] address.
    "call get_base",
    "mov ecx, [ecx]",

    // Load [`CallStorage`] address.
    "lea ecx, [ecx + {CALL_STORAGE_OFFSET}]",

    "mov eax, {UNMAP_FUNC_ID}",
    "mov [ecx + {CALL_STORAGE_FUNC_ID_OFFSET}], ax",

    "mov eax, 2",
    "mov [ecx + {CALL_STORAGE_ARG_COUNT_OFFSET}], al",

    "mov eax, [esp + 4]",
    "mov [ecx + {CALL_STORAGE_ARG_0_OFFSET}], eax",
    "xor eax, eax",
    "mov [ecx + {CALL_STORAGE_ARG_0_OFFSET} + 4], eax",

    "mov eax, [esp + 8]",
    "mov [ecx + {CALL_STORAGE_ARG_1_OFFSET}], eax",
    "xor eax, eax",
    "mov [ecx + {CALL_STORAGE_ARG_1_OFFSET} + 4], eax",

    "call transition",

    "ret",

    ".global X86_32_CODE_TAKEOVER",
    "X86_32_CODE_TAKEOVER:",

    // Load [`Storage`] address.
    "call get_base",
    "mov ecx, [ecx]",

    // Load [`CallStorage`] address.
    "lea ecx, [ecx + {CALL_STORAGE_OFFSET}]",

    "mov eax, {TAKEOVER_FUNC_ID}",
    "mov [ecx + {CALL_STORAGE_FUNC_ID_OFFSET}], ax",

    "mov eax, 3",
    "mov [ecx + {CALL_STORAGE_ARG_COUNT_OFFSET}], al",

    "mov eax, [esp + 4]",
    "mov [ecx + {CALL_STORAGE_ARG_0_OFFSET}], eax",
    "mov eax, [esp + 8]",
    "mov [ecx + {CALL_STORAGE_ARG_0_OFFSET} + 4], eax",

    "mov eax, [esp + 12]",
    "mov [ecx + {CALL_STORAGE_ARG_1_OFFSET}], eax",
    "mov eax, [esp + 16]",
    "mov [ecx + {CALL_STORAGE_ARG_1_OFFSET} + 4], eax",

    "mov eax, [esp + 20]",
    "mov [ecx + {CALL_STORAGE_ARG_2_OFFSET}], eax",
    "xor eax, eax",
    "mov [ecx + {CALL_STORAGE_ARG_2_OFFSET} + 4], eax",

    "call transition",

    "ret",

    ".global X86_32_CODE_RUN_ON_ALL_PROCESSORS",
    "X86_32_CODE_RUN_ON_ALL_PROCESSORS:",

    // Load [`Storage`] address.
    "call get_base",
    "mov ecx, [ecx]",

    // Load [`CallStorage`] address.
    "lea ecx, [ecx + {CALL_STORAGE_OFFSET}]",

    "mov eax, {RUN_ON_ALL_PROCESSORS_FUNC_ID}",
    "mov [ecx + {CALL_STORAGE_FUNC_ID_OFFSET}], ax",

    "mov eax, 2",
    "mov [ecx + {CALL_STORAGE_ARG_COUNT_OFFSET}], al",

    "mov eax, [esp + 4]",
    "mov [ecx + {CALL_STORAGE_ARG_0_OFFSET}], eax",
    "xor eax, eax",
    "mov [ecx + {CALL_STORAGE_ARG_0_OFFSET} + 4], eax",

    "mov eax, [esp + 8]",
    "mov [ecx + {CALL_STORAGE_ARG_1_OFFSET}], eax",
    "xor eax, eax",
    "mov [ecx + {CALL_STORAGE_ARG_1_OFFSET} + 4], eax",

    "call transition",

    "ret",

    ".global X86_32_CODE_END",
    "X86_32_CODE_END:",

    #[cfg(target_arch = "x86_64")]
    ".code64",

    // Function call IDs.
    RETURN_FUNC_ID = const { RETURN_FUNC_ID },

    // Executable to stub function call IDs.
    WRITE_FUNC_ID = const { WRITE_FUNC_ID },
    ALLOCATE_FRAMES_FUNC_ID = const { ALLOCATE_FRAMES_FUNC_ID },
    DEALLOCATE_FRAMES_FUNC_ID = const { DEALLOCATE_FRAMES_FUNC_ID },
    GET_MEMORY_MAP_FUNC_ID = const { GET_MEMORY_MAP_FUNC_ID },
    MAP_FUNC_ID = const { MAP_FUNC_ID },
    UNMAP_FUNC_ID = const { UNMAP_FUNC_ID },
    TAKEOVER_FUNC_ID = const { TAKEOVER_FUNC_ID },
    RUN_ON_ALL_PROCESSORS_FUNC_ID = const { RUN_ON_ALL_PROCESSORS_FUNC_ID },

    // Stub to executable function calls.
    ENTER_FUNC_ID = const { ENTER_FUNC_ID },
    EXEC_ON_PROCESSOR_FUNC_ID = const { EXEC_ON_PROCESSOR_FUNC_ID },

    // [`CpuStorage`] layout.
    CALL_STORAGE_OFFSET = const { offset_of!(CpuStorage, call) },

    STORAGE_CHANGE_EFER = const { offset_of!(CpuStorage, change_efer) },

    CALL_STORAGE_FUNC_ID_OFFSET = const { offset_of!(CallStorage, func_id) },
    CALL_STORAGE_ARG_COUNT_OFFSET = const { offset_of!(CallStorage, arg_count) },
    CALL_STORAGE_ARG_0_OFFSET = const { offset_of!(CallStorage, arg_0) },
    CALL_STORAGE_ARG_1_OFFSET = const { offset_of!(CallStorage, arg_1) },
    CALL_STORAGE_ARG_2_OFFSET = const { offset_of!(CallStorage, arg_2) },
    CALL_STORAGE_ARG_3_OFFSET = const { offset_of!(CallStorage, arg_3) },
    CALL_STORAGE_ARG_4_OFFSET = const { offset_of!(CallStorage, arg_4) },
    CALL_STORAGE_RET_OFFSET = const { offset_of!(CallStorage, ret) },

    MODE_STORAGE_HANDLE_ENTER_MODE = const { offset_of!(ModeStorage, enter_mode) },
    MODE_STORAGE_HANDLE_CALL_HANDLER = const { offset_of!(ModeStorage, call_handler) },

    MODE_STORAGE_RAX = const { offset_of!(ModeStorage, rax) },
    MODE_STORAGE_RBX = const { offset_of!(ModeStorage, rbx) },
    MODE_STORAGE_RCX = const { offset_of!(ModeStorage, rcx) },
    MODE_STORAGE_RDX = const { offset_of!(ModeStorage, rdx) },

    MODE_STORAGE_RSI = const { offset_of!(ModeStorage, rsi) },
    MODE_STORAGE_RDI = const { offset_of!(ModeStorage, rdi) },
    MODE_STORAGE_RSP = const { offset_of!(ModeStorage, rsp) },
    MODE_STORAGE_RBP = const { offset_of!(ModeStorage, rbp) },

    MODE_STORAGE_CS = const { offset_of!(ModeStorage, cs) },
    MODE_STORAGE_DS = const { offset_of!(ModeStorage, ds) },
    MODE_STORAGE_ES = const { offset_of!(ModeStorage, es) },
    MODE_STORAGE_FS = const { offset_of!(ModeStorage, fs) },
    MODE_STORAGE_GS = const { offset_of!(ModeStorage, gs) },
    MODE_STORAGE_SS = const { offset_of!(ModeStorage, ss) },

    MODE_STORAGE_CR0 = const { offset_of!(ModeStorage, cr0) },
    MODE_STORAGE_CR3 = const { offset_of!(ModeStorage, cr3) },
    MODE_STORAGE_CR4 = const { offset_of!(ModeStorage, cr4) },

    MODE_STORAGE_GDTR = const { offset_of!(ModeStorage, gdtr) },
    MODE_STORAGE_IDTR = const { offset_of!(ModeStorage, idtr) },

    MODE_STORAGE_EFER = const { offset_of!(ModeStorage, efer) },
}
