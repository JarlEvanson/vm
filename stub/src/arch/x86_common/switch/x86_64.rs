//! Implementation of cross-address space switching functionality for `x86_64`.

use core::{
    arch::global_asm,
    mem::{self, offset_of},
};

use crate::{
    arch::{
        ArchAddressSpace,
        generic::{
            address_space::{AddressSpace, ProtectionFlags},
            switch::{
                ALLOCATE_FRAMES_FUNC_ID, DEALLOCATE_FRAMES_FUNC_ID, GET_MEMORY_MAP_FUNC_ID,
                MAP_FUNC_ID, RETURN_FUNC_ID, TAKEOVER_FUNC_ID, UNMAP_FUNC_ID, WRITE_FUNC_ID,
            },
        },
        x86_common::switch::{
            CallStorage, CodeLayout, ComponentError, ModeStorage, PAGE_FAULT_ID,
            PROTECTED_MODE_POLICY, Storage, TablePointer,
        },
    },
    platform::{
        FrameAllocation, allocate_frames_aligned, frame_size, map_identity, write_bytes_at,
        write_u64_at,
    },
    util::usize_to_u64,
};

/// Allocates and maps the code switching.
pub fn allocate_code(
    address_space: &mut ArchAddressSpace,
    storage_ptr: u64,
    own_mode_storage_ptr: u64,
    other_mode_storage_ptr: u64,
) -> Result<CodeLayout, ComponentError> {
    let code_start_ptr = &raw const X86_64_CODE_START;
    let code_end_ptr = &raw const X86_64_CODE_END;
    let code_size = code_end_ptr.addr().strict_sub(code_start_ptr.addr());
    let code_size_u64 = usize_to_u64(code_size);

    let frame_allocation = allocate_frames_aligned(
        code_size_u64.div_ceil(frame_size()),
        address_space.page_size(),
        PROTECTED_MODE_POLICY,
    )?;

    // Map code region into the two address spaces.
    let _ = map_identity(
        frame_allocation.range().start().start_address(),
        code_size_u64,
    );
    address_space.map(
        frame_allocation.range().start().start_address().value(),
        frame_allocation.range().start().start_address().value(),
        code_size_u64.div_ceil(address_space.page_size()),
        ProtectionFlags::READ | ProtectionFlags::WRITE | ProtectionFlags::EXEC,
    )?;

    // Fill the code region with the provided code.

    // SAFETY:
    //
    // The allocated region is part of `revm-stub` and is not writable and thus it is safe to
    // create an immutable slice.
    let code_bytes = unsafe { core::slice::from_raw_parts(code_start_ptr, code_size) };
    write_bytes_at(frame_allocation.range().start().start_address(), code_bytes);

    // Fill the `storage_ptr` and `mode_storage_ptr` in the assembly code.
    write_u64_at(
        frame_allocation.range().start().start_address(),
        storage_ptr,
    );
    write_u64_at(
        frame_allocation.range().start().start_address().add(8),
        own_mode_storage_ptr,
    );
    write_u64_at(
        frame_allocation.range().start().start_address().add(16),
        other_mode_storage_ptr,
    );

    let offset = usize_to_u64(code_start_ptr.addr())
        .wrapping_sub(frame_allocation.range().start().start_address().value());

    let handle_call_internal_ptr = &raw const X86_64_CODE_HANDLE_CALL_INTERNAL;
    let handle_call_internal = usize_to_u64(handle_call_internal_ptr.addr()).wrapping_sub(offset);

    let handle_call_external_ptr = &raw const X86_64_CODE_HANDLE_CALL_EXTERNAL;
    let handle_call_external = usize_to_u64(handle_call_external_ptr.addr()).wrapping_sub(offset);

    let entry_ptr = &raw const X86_64_CODE_ENTRY;
    let entry = usize_to_u64(entry_ptr.addr()).wrapping_sub(offset);

    let write_ptr = &raw const X86_64_CODE_WRITE;
    let write = usize_to_u64(write_ptr.addr()).wrapping_sub(offset);

    let allocate_frames_ptr = &raw const X86_64_CODE_ALLOCATE_FRAMES;
    let allocate_frames = usize_to_u64(allocate_frames_ptr.addr()).wrapping_sub(offset);

    let deallocate_frames_ptr = &raw const X86_64_CODE_DEALLOCATE_FRAMES;
    let deallocate_frames = usize_to_u64(deallocate_frames_ptr.addr()).wrapping_sub(offset);

    let get_memory_map_ptr = &raw const X86_64_CODE_GET_MEMORY_MAP;
    let get_memory_map = usize_to_u64(get_memory_map_ptr.addr()).wrapping_sub(offset);

    let map_ptr = &raw const X86_64_CODE_MAP;
    let map = usize_to_u64(map_ptr.addr()).wrapping_sub(offset);

    let unmap_ptr = &raw const X86_64_CODE_UNMAP;
    let unmap = usize_to_u64(unmap_ptr.addr()).wrapping_sub(offset);

    let takeover_ptr = &raw const X86_64_CODE_TAKEOVER;
    let takeover = usize_to_u64(takeover_ptr.addr()).wrapping_sub(offset);

    let page_fault_handler_ptr = &raw const X86_64_CODE_PAGE_FAULT_HANDLER;
    let page_fault_handler = usize_to_u64(page_fault_handler_ptr.addr()).wrapping_sub(offset);

    let code_layout = CodeLayout {
        frame_allocation,

        handle_call_internal,
        handle_call_external,

        entry,
        write,
        allocate_frames,
        deallocate_frames,
        get_memory_map,
        map,
        unmap,
        takeover,

        page_fault_handler,
    };

    Ok(code_layout)
}

pub fn allocate_idt(
    address_space: &mut ArchAddressSpace,
    layout: &CodeLayout,
) -> Result<(FrameAllocation, TablePointer), ComponentError> {
    let idt_size = mem::size_of::<Idt>();
    let idt_size_u64 = usize_to_u64(idt_size);

    let frame_allocation = allocate_frames_aligned(
        idt_size_u64.div_ceil(frame_size()),
        address_space.page_size(),
        PROTECTED_MODE_POLICY,
    )?;

    address_space.map(
        frame_allocation.range().start().start_address().value(),
        frame_allocation.range().start().start_address().value(),
        idt_size_u64.div_ceil(address_space.page_size()),
        ProtectionFlags::READ | ProtectionFlags::WRITE,
    )?;

    let mut idt = Idt::new();

    idt.set_handler(14, layout.page_fault_handler, 24, 0x8F, 0);

    // SAFETY:
    //
    // TODO:
    let idt_bytes = unsafe { core::slice::from_raw_parts((&raw const idt).cast::<u8>(), idt_size) };
    write_bytes_at(frame_allocation.range().start().start_address(), idt_bytes);

    let idt_pointer = frame_allocation.range().start().start_address().value();
    #[expect(clippy::cast_possible_truncation)]
    Ok((
        frame_allocation,
        TablePointer {
            size: idt_size_u64 as u16,
            pointer: idt_pointer,
        },
    ))
}

#[repr(C, align(16))]
pub struct Idt {
    entries: [IdtEntry; 256],
}

impl Idt {
    pub const fn new() -> Self {
        Self {
            entries: [IdtEntry::missing(); 256],
        }
    }

    pub fn set_handler(
        &mut self,
        index: usize,
        handler: u64,
        selector: u16,
        attributes: u8,
        ist: u8,
    ) {
        self.entries[index] = IdtEntry::new(handler, selector, attributes, ist);
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    attributes: u8,
    offset_middle: u16,
    offset_high: u32,
    zero: u32,
}

impl IdtEntry {
    pub const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            attributes: 0,
            offset_middle: 0,
            offset_high: 0,
            zero: 0,
        }
    }

    pub fn new(handler: u64, selector: u16, attributes: u8, ist: u8) -> Self {
        let addr = handler as u64;
        Self {
            offset_low: addr as u16,
            selector,
            ist,
            attributes,
            offset_middle: (addr >> 16) as u16,
            offset_high: (addr >> 32) as u32,
            zero: 0,
        }
    }
}

unsafe extern "C" {
    static X86_64_CODE_START: u8;

    static X86_64_CODE_HANDLE_CALL_INTERNAL: u8;
    static X86_64_CODE_HANDLE_CALL_EXTERNAL: u8;

    static X86_64_CODE_ENTRY: u8;
    static X86_64_CODE_WRITE: u8;
    static X86_64_CODE_ALLOCATE_FRAMES: u8;
    static X86_64_CODE_DEALLOCATE_FRAMES: u8;
    static X86_64_CODE_GET_MEMORY_MAP: u8;
    static X86_64_CODE_MAP: u8;
    static X86_64_CODE_UNMAP: u8;
    static X86_64_CODE_TAKEOVER: u8;

    static X86_64_CODE_PAGE_FAULT_HANDLER: u8;

    static X86_64_CODE_END: u8;
}

global_asm! {
    #[cfg(target_arch = "x86")]
    ".code64",

    ".global X86_64_CODE_START",
    "X86_64_CODE_START:",

    // Allocate space for pointer to the [`Storage`] that this stub will use.
    "storage_ptr:",
    ".8byte 0",

    // Allocate space for pointer to the [`ModeStorage`] that this stub will use.
    "own_mode_storage_pointer:",
    ".8byte 0",

    // Allocate space for pointer to the [`ModeStorage`] that the other entity will use.
    "other_mode_storage_pointer:",
    ".8byte 0",

    // Calls the cross-address space function.
    //
    // `r10`: func_id
    // `r11`: `arg_count`
    //
    // Other registers must obey `x86_64` SysV calling convention.
    "call_internal:",

    "cmp r10, {RETURN_FUNC_ID}",
    "jne 5f",

    "mov r10, [rip + storage_ptr]",
    "mov [r10 + {CALL_STORAGE_OFFSET} + {CALL_STORAGE_RET_OFFSET}], rax",

    "xor rax, rax",
    "mov [r10 + {CALL_STORAGE_OFFSET} + {CALL_STORAGE_FUNC_ID_OFFSET}], ax",
    "jmp call_internal_continue",

    "5:",
    "push r11",
    "push r10",

    "mov r10, [rip + storage_ptr]",
    "mov [r10 + {CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_COUNT_OFFSET}], r11b",
    "mov r11, [rsp]",
    "mov [r10 + {CALL_STORAGE_OFFSET} + {CALL_STORAGE_FUNC_ID_OFFSET}], r11w",

    "pop r11",
    "pop r11",

    // Save the right number of arguments.
    "cmp r11, 0",
    "je call_internal_continue",

    "mov [r10 + {CALL_STORAGE_ARG_0_OFFSET}], rdi",
    "cmp r11, 1",
    "je call_internal_continue",

    "mov [r10 + {CALL_STORAGE_ARG_1_OFFSET}], rsi",
    "cmp r11, 2",
    "je call_internal_continue",

    "mov [r10 + {CALL_STORAGE_ARG_2_OFFSET}], rdx",
    "cmp r11, 3",
    "je call_internal_continue",

    "mov [r10 + {CALL_STORAGE_ARG_3_OFFSET}], rcx",
    "cmp r11, 4",
    "je call_internal_continue",

    "mov [r10 + {CALL_STORAGE_ARG_4_OFFSET}], r8",
    "cmp r11, 5",
    "je call_internal_continue",

    "mov [r10 + {CALL_STORAGE_ARG_5_OFFSET}], r9",
    "cmp r11, 6",
    "je call_internal_continue",

    "int 3",
    // Too many arguments, spin forever.
    "5:",
    "jmp 5b",

    "call_internal_continue:",

    // Save `rax` and `rbx` temporarily.
    "push rbx",
    "push rax",

    // Load the [`ModeStorage`] pointer into `rax`.
    "mov rax, [rip + own_mode_storage_pointer]",

    // Store `rax` and `rbx` into their slots.
    "mov [rax + {MODE_STORAGE_RBX}], rbx",
    "mov rbx, [rsp]",
    "mov [rax + {MODE_STORAGE_RAX}], rbx",

    "add rsp, 16",

    // Store the remainder of relevant registers.
    "mov [rax + {MODE_STORAGE_RCX}], rcx",
    "mov [rax + {MODE_STORAGE_RDX}], rdx",

    "mov [rax + {MODE_STORAGE_RSI}], rsi",
    "mov [rax + {MODE_STORAGE_RDI}], rdi",
    "mov [rax + {MODE_STORAGE_RSP}], rsp",
    "mov [rax + {MODE_STORAGE_RBP}], rbp",

    "mov [rax + {MODE_STORAGE_R8}], r8",
    "mov [rax + {MODE_STORAGE_R9}], r9",
    "mov [rax + {MODE_STORAGE_R10}], r10",
    "mov [rax + {MODE_STORAGE_R11}], r11",
    "mov [rax + {MODE_STORAGE_R12}], r12",
    "mov [rax + {MODE_STORAGE_R13}], r13",
    "mov [rax + {MODE_STORAGE_R14}], r14",
    "mov [rax + {MODE_STORAGE_R15}], r15",

    "mov bx, cs",
    "mov [rax + {MODE_STORAGE_CS}], bx",
    "mov bx, ds",
    "mov [rax + {MODE_STORAGE_DS}], bx",
    "mov bx, es",
    "mov [rax + {MODE_STORAGE_ES}], bx",
    "mov bx, fs",
    "mov [rax + {MODE_STORAGE_FS}], bx",
    "mov bx, gs",
    "mov [rax + {MODE_STORAGE_GS}], bx",
    "mov bx, ss",
    "mov [rax + {MODE_STORAGE_SS}], bx",

    "mov rbx, cr0",
    "mov [rax + {MODE_STORAGE_CR0}], rbx",
    "mov rbx, cr3",
    "mov [rax + {MODE_STORAGE_CR3}], rbx",
    "mov rbx, cr4",
    "mov [rax + {MODE_STORAGE_CR4}], rbx",

    "sgdt [rax + {MODE_STORAGE_GDTR}]",
    "sidt [rax + {MODE_STORAGE_IDTR}]",

    // All relevant registers have been stored; perform the switching now.

    // Load storage pointer.
    "mov rbx, [rip + storage_ptr]",

    // Construct a GDTR pointing to the stored `executable_gdt`.
    "lea rax, [rbx + {STORAGE_GDT}]",
    "push rax",

    // Compute the GDT size.
    "mov ax, 5 * 8 - 1",
    "push ax",

    // Load that GDT.
    "lgdt [rsp]",
    "add rsp, 10",

    // Load arguments for `X86_64_CODE_HANDLE_CALL_INTERNAL`.
    //
    // These arguments look reversed since the arguments are given from
    // the perspective of the other code.
    "mov rsp, [rip + other_mode_storage_pointer]",
    "mov rbp, [rip + own_mode_storage_pointer]",
    "mov rdi, [rip + storage_ptr]",

    "mov rbx, [rip + own_mode_storage_pointer]",
    "lea rcx, [rbx + {MODE_STORAGE_TMP_STORAGE}]",

    // Prepare the far jmp state.
    "lea rax, [rip + 5f]",
    "mov [rcx], rax",

    "mov rax, 8",
    "mov [rcx + 8], ax",

    // Load the CS32 segment register.
    "ljmp [rcx]",
    "5:",

    ".code32",

    // Load data segment registers.
    "mov ax, 16",
    "mov ds, ax",
    "mov es, ax",
    "mov fs, ax",
    "mov gs, ax",
    "mov ss, ax",

    // Get current `CR0` value.
    "mov ecx, cr0",

    // Disable paging bit.
    "mov edx, ~(1 << 31)",
    "and ecx, edx",

    // Set current `CR0` value.
    "mov cr0, ecx",

    "mov ecx, [esp + {MODE_STORAGE_HANDLE_CALL_INTERNAL}]",
    "jmp ecx",

    ".code64",

    // Arguments:
    //
    // `esp`: own mode storage pointer.
    // `ebp`: other mode storage pointer.
    // `edi`: storage pointer.
    ".global X86_64_CODE_HANDLE_CALL_INTERNAL",
    "X86_64_CODE_HANDLE_CALL_INTERNAL:",

    // Handle call internal is always 32-bit mode (to enable paging-mode switching).
    ".code32",

    // Load control registers (this configures the CPU for long mode).
    "mov ecx, [esp + {MODE_STORAGE_CR3}]",
    "mov cr3, ecx",

    "mov ecx, [esp + {MODE_STORAGE_CR4}]",
    "mov cr4, ecx",

    // Switch to compatibility mode.
    "mov ecx, [esp + {MODE_STORAGE_CR0}]",
    "mov cr0, ecx",

    // Load pointer to temporary storage.
    "lea ecx, [esp + {MODE_STORAGE_TMP_STORAGE}]",

    // Construct a GDTR pointing to the stored `executable_gdt`.

    // First, compute the GDT size.
    "mov ax, 5 * 8 - 1",
    "mov [ecx], ax",

    // Then, obtain the address of the GDT.
    "lea eax, [edi + {STORAGE_GDT}]",
    "mov [ecx + 2], eax",

    "lgdt [ecx]",

    // Load data registers.
    "mov ax, 32",
    "mov ds, ax",
    "mov es, ax",
    "mov fs, ax",
    "mov gs, ax",
    "mov ss, ax",

    "lea esp, [ecx + 16]",

    // Load CS64 register.
    "xor eax, eax",
    "mov ax, 24",
    "push eax",

    "call 5f",
    "5:",

    ".equ a_offset, 5f - 5b",

    "pop eax",
    "add eax, offset a_offset",

    "push eax",
    "retf",
    "5:",

    ".code64",

    "mov rbx, [rip + own_mode_storage_pointer]",

    // Load IDT/GDT
    "lgdt [rbx + {MODE_STORAGE_GDTR}]",
    "lidt [rbx + {MODE_STORAGE_IDTR}]",

    // Prepare the far jmp state.
    "lea rax, [rip + 5f]",
    "mov [rcx], rax",

    "mov ax, [rbx + {MODE_STORAGE_CS}]",
    "mov [rcx + 8], ax",

    // Load the code segment register.
    "ljmp [rcx]",
    "5:",

    // Load data segment registers.
    "mov ax, [rbx + {MODE_STORAGE_DS}]",
    "mov ds, ax",
    "mov ax, [rbx + {MODE_STORAGE_ES}]",
    "mov es, ax",
    "mov ax, [rbx + {MODE_STORAGE_FS}]",
    "mov fs, ax",
    "mov ax, [rbx + {MODE_STORAGE_GS}]",
    "mov gs, ax",
    "mov ax, [rbx + {MODE_STORAGE_SS}]",
    "mov ss, ax",

    "mov r15, rbx",

    // Load general purpose registers.
    "mov rax, [r15 + {MODE_STORAGE_RAX}]",
    "mov rbx, [r15 + {MODE_STORAGE_RBX}]",
    "mov rcx, [r15 + {MODE_STORAGE_RCX}]",
    "mov rdx, [r15 + {MODE_STORAGE_RDX}]",

    "mov rsi, [r15 + {MODE_STORAGE_RSI}]",
    "mov rdi, [r15 + {MODE_STORAGE_RDI}]",
    "mov rsp, [r15 + {MODE_STORAGE_RSP}]",
    "mov rbp, [r15 + {MODE_STORAGE_RBP}]",

    "mov r8, [r15 + {MODE_STORAGE_R8}]",
    "mov r9, [r15 + {MODE_STORAGE_R9}]",
    "mov r10, [r15 + {MODE_STORAGE_R10}]",
    "mov r11, [r15 + {MODE_STORAGE_R11}]",
    "mov r12, [r15 + {MODE_STORAGE_R12}]",
    "mov r13, [r15 + {MODE_STORAGE_R13}]",
    "mov r14, [r15 + {MODE_STORAGE_R14}]",
    "mov r15, [r15 + {MODE_STORAGE_R15}]",

    // Load [`CallStorage`].
    "mov r10, [rip + storage_ptr]",
    "lea r10, [r10 + {CALL_STORAGE_OFFSET}]",

    // Determine whether to return or to handle the call.
    "mov r11w, [r10 + {CALL_STORAGE_FUNC_ID_OFFSET}]",
    "cmp r11w, 0",
    "jne 5f",

    // Return.
    "mov rax, [r10 + {CALL_STORAGE_RET_OFFSET}]",
    "ret",

    // Handle a call.
    "5:",

    "mov r10, [rip + own_mode_storage_pointer]",
    "mov r10, [r10 + {MODE_STORAGE_HANDLE_CALL_EXTERNAL}]",

    "call r10",

    "mov r10, {RETURN_FUNC_ID}",
    "mov r11, 1",

    "jmp call_internal",
    "5:", "jmp 5b",

    ".global X86_64_CODE_HANDLE_CALL_EXTERNAL",
    "X86_64_CODE_HANDLE_CALL_EXTERNAL:",

    "mov r10, [rip + storage_ptr]",

    // Load the only argument to the only function that executable provides.
    "mov rdi, [r10 + {CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_0_OFFSET}]",

    "mov r10, [r10 + {STORAGE_EXECUTABLE_ENTRY_POINT}]",

    "call r10",

    "ret",

    ".global X86_64_CODE_ENTRY",
    "X86_64_CODE_ENTRY:",

    "call call_internal_continue",

    "ret",

    ".global X86_64_CODE_WRITE",
    "X86_64_CODE_WRITE:",

    "mov r10, {WRITE_FUNC_ID}",
    "mov r11, 2",

    "call call_internal",

    "ret",

    ".global X86_64_CODE_ALLOCATE_FRAMES",
    "X86_64_CODE_ALLOCATE_FRAMES:",

    "mov r10, {ALLOCATE_FRAMES_FUNC_ID}",
    "mov r11, 4",

    "call call_internal",

    "ret",

    ".global X86_64_CODE_DEALLOCATE_FRAMES",
    "X86_64_CODE_DEALLOCATE_FRAMES:",

    "mov r10, {DEALLOCATE_FRAMES_FUNC_ID}",
    "mov r11, 2",

    "call call_internal",

    "ret",

    ".global X86_64_CODE_GET_MEMORY_MAP",
    "X86_64_CODE_GET_MEMORY_MAP:",

    "mov r10, {GET_MEMORY_MAP_FUNC_ID}",
    "mov r11, 5",

    "call call_internal",

    "ret",

    ".global X86_64_CODE_MAP",
    "X86_64_CODE_MAP:",

    "mov r10, {MAP_FUNC_ID}",
    "mov r11, 4",

    "call call_internal",

    "ret",

    ".global X86_64_CODE_UNMAP",
    "X86_64_CODE_UNMAP:",

    "mov r10, {UNMAP_FUNC_ID}",
    "mov r11, 2",

    "call call_internal",

    "ret",

    ".global X86_64_CODE_TAKEOVER",
    "X86_64_CODE_TAKEOVER:",

    "mov r10, {TAKEOVER_FUNC_ID}",
    "mov r11, 2",

    "call call_internal",

    "ret",

    ".global X86_64_CODE_PAGE_FAULT_HANDLER",
    "X86_64_CODE_PAGE_FAULT_HANDLER:",

    "mov r10, {PAGE_FAULT_ID}",
    "mov r11, 6",

    "mov rdi, [rsp]",
    "mov rsi, [rsp + 8]",
    "mov rdx, [rsp + 16]",
    "mov rcx, [rsp + 24]",
    "mov r8, [rsp + 32]",
    "mov r9, [rsp + 40]",

    "call call_internal",

    "ret",

    ".global X86_64_CODE_END",
    "X86_64_CODE_END:",

    #[cfg(target_arch = "x86")]
    ".code32",

    RETURN_FUNC_ID = const { RETURN_FUNC_ID },
    WRITE_FUNC_ID = const { WRITE_FUNC_ID },
    ALLOCATE_FRAMES_FUNC_ID = const { ALLOCATE_FRAMES_FUNC_ID },
    DEALLOCATE_FRAMES_FUNC_ID = const { DEALLOCATE_FRAMES_FUNC_ID },
    GET_MEMORY_MAP_FUNC_ID = const { GET_MEMORY_MAP_FUNC_ID },
    MAP_FUNC_ID = const { MAP_FUNC_ID },
    UNMAP_FUNC_ID = const { UNMAP_FUNC_ID },
    TAKEOVER_FUNC_ID = const { TAKEOVER_FUNC_ID },

    CALL_STORAGE_OFFSET = const { offset_of!(Storage, call) },

    CALL_STORAGE_FUNC_ID_OFFSET = const { offset_of!(CallStorage, func_id) },
    CALL_STORAGE_ARG_COUNT_OFFSET = const { offset_of!(CallStorage, arg_count) },
    CALL_STORAGE_ARG_0_OFFSET = const { offset_of!(CallStorage, arg_0) },
    CALL_STORAGE_ARG_1_OFFSET = const { offset_of!(CallStorage, arg_1) },
    CALL_STORAGE_ARG_2_OFFSET = const { offset_of!(CallStorage, arg_2) },
    CALL_STORAGE_ARG_3_OFFSET = const { offset_of!(CallStorage, arg_3) },
    CALL_STORAGE_ARG_4_OFFSET = const { offset_of!(CallStorage, arg_4) },
    CALL_STORAGE_ARG_5_OFFSET = const { offset_of!(CallStorage, arg_5) },
    CALL_STORAGE_RET_OFFSET = const { offset_of!(CallStorage, ret) },

    MODE_STORAGE_HANDLE_CALL_INTERNAL = const { offset_of!(ModeStorage, handle_call_internal) },
    MODE_STORAGE_HANDLE_CALL_EXTERNAL = const { offset_of!(ModeStorage, handle_call_external) },

    MODE_STORAGE_RAX = const { offset_of!(ModeStorage, rax) },
    MODE_STORAGE_RBX = const { offset_of!(ModeStorage, rbx) },
    MODE_STORAGE_RCX = const { offset_of!(ModeStorage, rcx) },
    MODE_STORAGE_RDX = const { offset_of!(ModeStorage, rdx) },

    MODE_STORAGE_RSI = const { offset_of!(ModeStorage, rsi) },
    MODE_STORAGE_RDI = const { offset_of!(ModeStorage, rdi) },
    MODE_STORAGE_RSP = const { offset_of!(ModeStorage, rsp) },
    MODE_STORAGE_RBP = const { offset_of!(ModeStorage, rbp) },
    MODE_STORAGE_R8  = const { offset_of!(ModeStorage, r8) },
    MODE_STORAGE_R9  = const { offset_of!(ModeStorage, r9) },
    MODE_STORAGE_R10 = const { offset_of!(ModeStorage, r10) },
    MODE_STORAGE_R11 = const { offset_of!(ModeStorage, r11) },
    MODE_STORAGE_R12 = const { offset_of!(ModeStorage, r12) },
    MODE_STORAGE_R13 = const { offset_of!(ModeStorage, r13) },
    MODE_STORAGE_R14 = const { offset_of!(ModeStorage, r14) },
    MODE_STORAGE_R15 = const { offset_of!(ModeStorage, r15) },

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

    MODE_STORAGE_TMP_STORAGE = const { offset_of!(ModeStorage, tmp_storage) },

    STORAGE_EXECUTABLE_ENTRY_POINT = const { offset_of!(Storage, executable_entry_point) },
    STORAGE_GDT = const { offset_of!(Storage, executable_gdt) },

    PAGE_FAULT_ID = const { PAGE_FAULT_ID },
}
