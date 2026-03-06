//! Implementation of cross-address space switching functionality for `x86_32`.

use core::{arch::global_asm, mem::offset_of};

use conversion::usize_to_u64;
use memory::{
    address::{AddressChunk, AddressChunkRange, PhysicalAddressRange},
    phys::PhysicalMemorySpace,
    translation::{MapFlags, TranslationScheme},
};

use crate::{
    arch::{
        aarch64::switch::{CallStorage, ModeStorage},
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
    let code_start_ptr = &raw const AARCH64_CODE_START;
    let code_end_ptr = &raw const AARCH64_CODE_END;
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

    let enter_mode_ptr = &raw const AARCH64_CODE_ENTER_MODE;
    let enter_mode = usize_to_u64(enter_mode_ptr.addr()).wrapping_sub(offset);

    let call_handler_ptr = &raw const AARCH64_CODE_CALL_HANDLER;
    let call_handler = usize_to_u64(call_handler_ptr.addr()).wrapping_sub(offset);

    let call_ptr = &raw const AARCH64_CODE_CALL;
    let call = usize_to_u64(call_ptr.addr()).wrapping_sub(offset);

    let write_ptr = &raw const AARCH64_CODE_WRITE;
    let write = usize_to_u64(write_ptr.addr()).wrapping_sub(offset);

    let allocate_frames_ptr = &raw const AARCH64_CODE_ALLOCATE_FRAMES;
    let allocate_frames = usize_to_u64(allocate_frames_ptr.addr()).wrapping_sub(offset);

    let deallocate_frames_ptr = &raw const AARCH64_CODE_DEALLOCATE_FRAMES;
    let deallocate_frames = usize_to_u64(deallocate_frames_ptr.addr()).wrapping_sub(offset);

    let get_memory_map_ptr = &raw const AARCH64_CODE_GET_MEMORY_MAP;
    let get_memory_map = usize_to_u64(get_memory_map_ptr.addr()).wrapping_sub(offset);

    let map_ptr = &raw const AARCH64_CODE_MAP;
    let map = usize_to_u64(map_ptr.addr()).wrapping_sub(offset);

    let unmap_ptr = &raw const AARCH64_CODE_UNMAP;
    let unmap = usize_to_u64(unmap_ptr.addr()).wrapping_sub(offset);

    let takeover_ptr = &raw const AARCH64_CODE_TAKEOVER;
    let takeover = usize_to_u64(takeover_ptr.addr()).wrapping_sub(offset);

    let run_on_all_processors_ptr = &raw const AARCH64_CODE_RUN_ON_ALL_PROCESSORS;
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
    static AARCH64_CODE_START: u8;

    static AARCH64_CODE_ENTER_MODE: u8;

    static AARCH64_CODE_CALL_HANDLER: u8;
    static AARCH64_CODE_CALL: u8;

    static AARCH64_CODE_WRITE: u8;
    static AARCH64_CODE_ALLOCATE_FRAMES: u8;
    static AARCH64_CODE_DEALLOCATE_FRAMES: u8;
    static AARCH64_CODE_GET_MEMORY_MAP: u8;
    static AARCH64_CODE_MAP: u8;
    static AARCH64_CODE_UNMAP: u8;
    static AARCH64_CODE_TAKEOVER: u8;
    static AARCH64_CODE_RUN_ON_ALL_PROCESSORS: u8;

    static AARCH64_CODE_END: u8;
}

global_asm! {
    ".global AARCH64_CODE_START",
    "AARCH64_CODE_START:",

    // Allocate space for pointer to the [`Storage`] that this stub will use.
    "storage_pointer:",
    ".8byte 0",

    // Allocate space for pointer to the [`ModeStorage`] that this stub will use.
    "own_mode_storage_pointer:",
    ".8byte 0",

    // Allocate space for pointer to the [`ModeStorage`] that the other entity will use.
    "other_mode_storage_pointer:",
    ".8byte 0",

    "transition:",

    "adr x16, storage_pointer",
    "ldr x16, [x16, own_mode_storage_pointer - storage_pointer]",

    "str x0, [x16, #{MODE_STORAGE_X0}]",
    "str x1, [x16, #{MODE_STORAGE_X1}]",
    "str x2, [x16, #{MODE_STORAGE_X2}]",
    "str x3, [x16, #{MODE_STORAGE_X3}]",
    "str x4, [x16, #{MODE_STORAGE_X4}]",
    "str x5, [x16, #{MODE_STORAGE_X5}]",
    "str x6, [x16, #{MODE_STORAGE_X6}]",
    "str x7, [x16, #{MODE_STORAGE_X7}]",
    "str x8, [x16, #{MODE_STORAGE_X8}]",
    "str x9, [x16, #{MODE_STORAGE_X9}]",
    "str x10, [x16, #{MODE_STORAGE_X10}]",
    "str x11, [x16, #{MODE_STORAGE_X11}]",
    "str x12, [x16, #{MODE_STORAGE_X12}]",
    "str x13, [x16, #{MODE_STORAGE_X13}]",
    "str x14, [x16, #{MODE_STORAGE_X14}]",
    "str x15, [x16, #{MODE_STORAGE_X15}]",
    "str x16, [x16, #{MODE_STORAGE_X16}]",
    "str x17, [x16, #{MODE_STORAGE_X17}]",
    "str x18, [x16, #{MODE_STORAGE_X18}]",
    "str x19, [x16, #{MODE_STORAGE_X19}]",
    "str x20, [x16, #{MODE_STORAGE_X20}]",
    "str x21, [x16, #{MODE_STORAGE_X21}]",
    "str x22, [x16, #{MODE_STORAGE_X22}]",
    "str x23, [x16, #{MODE_STORAGE_X23}]",
    "str x24, [x16, #{MODE_STORAGE_X24}]",
    "str x25, [x16, #{MODE_STORAGE_X25}]",
    "str x26, [x16, #{MODE_STORAGE_X26}]",
    "str x27, [x16, #{MODE_STORAGE_X27}]",
    "str x28, [x16, #{MODE_STORAGE_X28}]",
    "str x29, [x16, #{MODE_STORAGE_X29}]",
    "str x30, [x16, #{MODE_STORAGE_X30}]",

    "mov x17, sp",
    "str x17, [x16, #{MODE_STORAGE_SP}]",

    "mrs x17, CurrentEL",
    "lsr x17, x17, #2",
    "cmp x17, #2",
    "bne 5f",

    // EL2.
    "mrs x17, tcr_el2",
    "str x17, [x16, #{MODE_STORAGE_TCR_ELX}]",

    "mrs x17, ttbr0_el2",
    "str x17, [x16, #{MODE_STORAGE_TTBR0_ELX}]",

    "mrs x17, s3_0_c2_c1_1", // Raw sysreg encoding of `ttbr1_el2`.
    "str x17, [x16, #{MODE_STORAGE_TTBR1_ELX}]",

    "mrs x17, mair_el2",
    "str x17, [x16, #{MODE_STORAGE_MAIR_ELX}]",

    "mrs x17, sctlr_el2",
    "str x17, [x16, #{MODE_STORAGE_SCTLR_ELX}]",

    "mrs x17, vbar_el2",
    "str x17, [x16, #{MODE_STORAGE_VBAR_ELX}]",

    "mrs x17, elr_el2",
    "str x17, [x16, #{MODE_STORAGE_ELR_ELX}]",

    "mrs x17, spsr_el2",
    "str x17, [x16, #{MODE_STORAGE_SPSR_ELX}]",

    // Disable MMU bit.
    "mrs x17, sctlr_el2",
    "bic x17, x17, #1",
    "msr sctlr_el2, x17",

    "dsb ish",
    "isb",

    "adr x16, storage_pointer",
    "ldr x16, [x16, other_mode_storage_pointer - storage_pointer]",
    "ldr x17, [x16, #{MODE_STORAGE_ENTER_MODE}]",
    "br x17",

    "5:",

    // EL1.
    "mrs x17, tcr_el1",
    "str x17, [x16, #{MODE_STORAGE_TCR_ELX}]",

    "mrs x17, ttbr0_el1",
    "str x17, [x16, #{MODE_STORAGE_TTBR0_ELX}]",

    "mrs x17, ttbr1_el1",
    "str x17, [x16, #{MODE_STORAGE_TTBR1_ELX}]",

    "mrs x17, mair_el1",
    "str x17, [x16, #{MODE_STORAGE_MAIR_ELX}]",

    "mrs x17, sctlr_el1",
    "str x17, [x16, #{MODE_STORAGE_SCTLR_ELX}]",

    "mrs x17, vbar_el1",
    "str x17, [x16, #{MODE_STORAGE_VBAR_ELX}]",

    "mrs x17, elr_el1",
    "str x17, [x16, #{MODE_STORAGE_ELR_ELX}]",

    "mrs x17, spsr_el1",
    "str x17, [x16, #{MODE_STORAGE_SPSR_ELX}]",

    // Disable MMU bit.
    "mrs x17, sctlr_el1",
    "bic x17, x17, #1",
    "msr sctlr_el1, x17",

    "dsb ish",
    "isb",

    "adr x16, storage_pointer",
    "ldr x16, [x16, other_mode_storage_pointer - storage_pointer]",
    "ldr x17, [x16, #{MODE_STORAGE_ENTER_MODE}]",
    "br x17",

    ".global AARCH64_CODE_ENTER_MODE",
    "AARCH64_CODE_ENTER_MODE:",

    "adr x16, storage_pointer",
    "ldr x16, [x16, own_mode_storage_pointer - storage_pointer]",

    "mrs x17, CurrentEL",
    "lsr x17, x17, #2",
    "cmp x17, #2",
    "bne 5f",

    // EL2.
    "ldr x17, [x16, #{MODE_STORAGE_TCR_ELX}]",
    "msr tcr_el2, x17",

    "ldr x17, [x16, #{MODE_STORAGE_TTBR0_ELX}]",
    "msr ttbr0_el2, x17",

    "ldr x17, [x16, #{MODE_STORAGE_TTBR1_ELX}]",
    "msr s3_0_c2_c1_1, x17", // Raw sysreg encoding of `ttbr1_el2`.

    "ldr x17, [x16, #{MODE_STORAGE_MAIR_ELX}]",
    "msr mair_el2, x17",

    "ldr x17, [x16, #{MODE_STORAGE_VBAR_ELX}]",
    "msr vbar_el2, x17",

    "ldr x17, [x16, #{MODE_STORAGE_ELR_ELX}]",
    "msr elr_el2, x17",

    "ldr x17, [x16, #{MODE_STORAGE_SPSR_ELX}]",
    "msr spsr_el2, x17",

    "dsb ish",
    "isb",

    "ldr x17, [x16, #{MODE_STORAGE_SCTLR_ELX}]",
    "msr sctlr_el2, x17",

    "isb",

    "b enter_mode_done",

    "5:",

    // EL1.
    "ldr x17, [x16, #{MODE_STORAGE_TCR_ELX}]",
    "msr tcr_el1, x17",

    "ldr x17, [x16, #{MODE_STORAGE_TTBR0_ELX}]",
    "msr ttbr0_el1, x17",

    "ldr x17, [x16, #{MODE_STORAGE_TTBR1_ELX}]",
    "msr ttbr1_el1, x17",

    "ldr x17, [x16, #{MODE_STORAGE_MAIR_ELX}]",
    "msr mair_el1, x17",

    "ldr x17, [x16, #{MODE_STORAGE_VBAR_ELX}]",
    "msr vbar_el1, x17",

    "ldr x17, [x16, #{MODE_STORAGE_ELR_ELX}]",
    "msr elr_el1, x17",

    "ldr x17, [x16, #{MODE_STORAGE_SPSR_ELX}]",
    "msr spsr_el1, x17",

    "dsb ish",
    "isb",

    "ldr x17, [x16, #{MODE_STORAGE_SCTLR_ELX}]",
    "msr sctlr_el1, x17",

    "isb",

    "enter_mode_done:",

    "ldr x0, [x16, #{MODE_STORAGE_X0}]",
    "ldr x1, [x16, #{MODE_STORAGE_X1}]",
    "ldr x2, [x16, #{MODE_STORAGE_X2}]",
    "ldr x3, [x16, #{MODE_STORAGE_X3}]",
    "ldr x4, [x16, #{MODE_STORAGE_X4}]",
    "ldr x5, [x16, #{MODE_STORAGE_X5}]",
    "ldr x6, [x16, #{MODE_STORAGE_X6}]",
    "ldr x7, [x16, #{MODE_STORAGE_X7}]",
    "ldr x8, [x16, #{MODE_STORAGE_X8}]",
    "ldr x9, [x16, #{MODE_STORAGE_X9}]",
    "ldr x10, [x16, #{MODE_STORAGE_X10}]",
    "ldr x11, [x16, #{MODE_STORAGE_X11}]",
    "ldr x12, [x16, #{MODE_STORAGE_X12}]",
    "ldr x13, [x16, #{MODE_STORAGE_X13}]",
    "ldr x14, [x16, #{MODE_STORAGE_X14}]",
    "ldr x15, [x16, #{MODE_STORAGE_X15}]",
    // Don't load `x16`, it is in active use.
    "ldr x17, [x16, #{MODE_STORAGE_X17}]",
    "ldr x18, [x16, #{MODE_STORAGE_X18}]",
    "ldr x19, [x16, #{MODE_STORAGE_X19}]",
    "ldr x20, [x16, #{MODE_STORAGE_X20}]",
    "ldr x21, [x16, #{MODE_STORAGE_X21}]",
    "ldr x22, [x16, #{MODE_STORAGE_X22}]",
    "ldr x23, [x16, #{MODE_STORAGE_X23}]",
    "ldr x24, [x16, #{MODE_STORAGE_X24}]",
    "ldr x25, [x16, #{MODE_STORAGE_X25}]",
    "ldr x26, [x16, #{MODE_STORAGE_X26}]",
    "ldr x27, [x16, #{MODE_STORAGE_X27}]",
    "ldr x28, [x16, #{MODE_STORAGE_X28}]",
    "ldr x29, [x16, #{MODE_STORAGE_X29}]",
    "ldr x30, [x16, #{MODE_STORAGE_X30}]",

    "ldr x17, [x16, #{MODE_STORAGE_SP}]",
    "mov sp, x17",

    "adr x16, storage_pointer",
    "ldr x16, [x16]",

    "ldrh w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_FUNC_ID_OFFSET}]",
    "cmp x17, #0",
    "bne 5f",

    // Return.
    "ldr x0, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_RET_OFFSET}]",
    "ret",

    "5:",

    // Handle a call.
    "adr x16, storage_pointer",
    "ldr x16, [x16, own_mode_storage_pointer - storage_pointer]",
    "ldr x17, [x16, #{MODE_STORAGE_CALL_HANDLER}]",

    "stp x29, x30, [sp, #-16]!",
    "mov x29, sp",

    "blr x17",

    "ldp x29, x30, [sp], #16",

    "adr x16, storage_pointer",
    "ldr x16, [x16]",

    "str x0, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_RET_OFFSET}]",

    "mov x17, #{RETURN_FUNC_ID}",
    "strh w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_FUNC_ID_OFFSET}]",

    "b transition",

    ".global AARCH64_CODE_CALL_HANDLER",
    "AARCH64_CODE_CALL_HANDLER:",

    "adr x16, storage_pointer",
    "ldr x16, [x16]",

    "ldrh w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_FUNC_ID_OFFSET}]",
    "cmp x17, #{ENTER_FUNC_ID}",
    "bne 5f",

    // When [`ENTER_FUNC_ID`] is called, the first argument is the entry point and the second is
    // the first argument to the entry point.
    "ldr x17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_0_OFFSET}]",
    "ldr x0, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_1_OFFSET}]",

    "br x17",

    "5:",
    "cmp x17, #{EXEC_ON_PROCESSOR_FUNC_ID}",
    "bne 5f",

    // When [`EXEC_ON_PROCESSOR_FUNC_ID`] is called, the first argument is the entry point, the
    // second argument is the CPU ID, and the third argument is the argument provided to each
    // processor.
    "ldr x17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_0_OFFSET}]",
    "ldr x0, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_1_OFFSET}]",
    "ldr x1, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_2_OFFSET}]",

    "br x17",

    "5:",
    "b 5b",

    ".global AARCH64_CODE_CALL",
    "AARCH64_CODE_CALL:",

    "b transition",

    ".global AARCH64_CODE_WRITE",
    "AARCH64_CODE_WRITE:",

    "adr x16, storage_pointer",
    "ldr x16, [x16]",

    "str x0, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_0_OFFSET}]",
    "str x1, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_1_OFFSET}]",

    "mov x17, #{WRITE_FUNC_ID}",
    "strh w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_FUNC_ID_OFFSET}]",

    "mov x17, 2",
    "strb w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_COUNT_OFFSET}]",

    "b transition",

    ".global AARCH64_CODE_ALLOCATE_FRAMES",
    "AARCH64_CODE_ALLOCATE_FRAMES:",

    "adr x16, storage_pointer",
    "ldr x16, [x16]",

    "str x0, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_0_OFFSET}]",
    "str x1, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_1_OFFSET}]",
    "str x2, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_2_OFFSET}]",
    "str x3, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_3_OFFSET}]",

    "mov x17, #{ALLOCATE_FRAMES_FUNC_ID}",
    "strh w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_FUNC_ID_OFFSET}]",

    "mov x17, 4",
    "strb w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_COUNT_OFFSET}]",

    "b transition",

    ".global AARCH64_CODE_DEALLOCATE_FRAMES",
    "AARCH64_CODE_DEALLOCATE_FRAMES:",

    "adr x16, storage_pointer",
    "ldr x16, [x16]",

    "str x0, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_0_OFFSET}]",
    "str x1, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_1_OFFSET}]",

    "mov x17, #{DEALLOCATE_FRAMES_FUNC_ID}",
    "strh w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_FUNC_ID_OFFSET}]",

    "mov x17, 2",
    "strb w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_COUNT_OFFSET}]",

    "b transition",

    ".global AARCH64_CODE_GET_MEMORY_MAP",
    "AARCH64_CODE_GET_MEMORY_MAP:",

    "adr x16, storage_pointer",
    "ldr x16, [x16]",

    "str x0, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_0_OFFSET}]",
    "str x1, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_1_OFFSET}]",
    "str x2, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_2_OFFSET}]",
    "str x3, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_3_OFFSET}]",
    "str x4, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_4_OFFSET}]",

    "mov x17, #{GET_MEMORY_MAP_FUNC_ID}",
    "strh w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_FUNC_ID_OFFSET}]",

    "mov x17, 5",
    "strb w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_COUNT_OFFSET}]",

    "b transition",

    ".global AARCH64_CODE_MAP",
    "AARCH64_CODE_MAP:",

    "adr x16, storage_pointer",
    "ldr x16, [x16]",

    "str x0, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_0_OFFSET}]",
    "str x1, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_1_OFFSET}]",
    "str x2, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_2_OFFSET}]",
    "str x3, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_3_OFFSET}]",

    "mov x17, #{MAP_FUNC_ID}",
    "strh w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_FUNC_ID_OFFSET}]",

    "mov x17, 4",
    "strb w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_COUNT_OFFSET}]",

    "b transition",

    ".global AARCH64_CODE_UNMAP",
    "AARCH64_CODE_UNMAP:",

    "adr x16, storage_pointer",
    "ldr x16, [x16]",

    "str x0, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_0_OFFSET}]",
    "str x1, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_1_OFFSET}]",

    "mov x17, #{UNMAP_FUNC_ID}",
    "strh w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_FUNC_ID_OFFSET}]",

    "mov x17, 2",
    "strb w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_COUNT_OFFSET}]",

    "b transition",

    ".global AARCH64_CODE_TAKEOVER",
    "AARCH64_CODE_TAKEOVER:",

    "adr x16, storage_pointer",
    "ldr x16, [x16]",

    "str x0, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_0_OFFSET}]",
    "str x1, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_1_OFFSET}]",
    "str x2, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_2_OFFSET}]",

    "mov x17, #{TAKEOVER_FUNC_ID}",
    "strh w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_FUNC_ID_OFFSET}]",

    "mov x17, 3",
    "strb w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_COUNT_OFFSET}]",

    "b transition",

    ".global AARCH64_CODE_RUN_ON_ALL_PROCESSORS",
    "AARCH64_CODE_RUN_ON_ALL_PROCESSORS:",

    "adr x16, storage_pointer",
    "ldr x16, [x16]",

    "str x0, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_0_OFFSET}]",
    "str x1, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_1_OFFSET}]",

    "mov x17, #{RUN_ON_ALL_PROCESSORS_FUNC_ID}",
    "strh w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_FUNC_ID_OFFSET}]",

    "mov x17, 2",
    "strb w17, [x16, #{CALL_STORAGE_OFFSET} + {CALL_STORAGE_ARG_COUNT_OFFSET}]",

    "b transition",

    ".global AARCH64_CODE_END",
    "AARCH64_CODE_END:",

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

    CALL_STORAGE_OFFSET = const { offset_of!(CpuStorage, call) },

    CALL_STORAGE_FUNC_ID_OFFSET = const { offset_of!(CallStorage, func_id) },
    CALL_STORAGE_ARG_COUNT_OFFSET = const { offset_of!(CallStorage, arg_count) },
    CALL_STORAGE_ARG_0_OFFSET = const { offset_of!(CallStorage, arg_0) },
    CALL_STORAGE_ARG_1_OFFSET = const { offset_of!(CallStorage, arg_1) },
    CALL_STORAGE_ARG_2_OFFSET = const { offset_of!(CallStorage, arg_2) },
    CALL_STORAGE_ARG_3_OFFSET = const { offset_of!(CallStorage, arg_3) },
    CALL_STORAGE_ARG_4_OFFSET = const { offset_of!(CallStorage, arg_4) },
    CALL_STORAGE_RET_OFFSET = const { offset_of!(CallStorage, ret) },

    MODE_STORAGE_ENTER_MODE = const { offset_of!(ModeStorage, enter_mode) },
    MODE_STORAGE_CALL_HANDLER = const { offset_of!(ModeStorage, call_handler) },

    MODE_STORAGE_X0 = const { offset_of!(ModeStorage, x0) },
    MODE_STORAGE_X1 = const { offset_of!(ModeStorage, x1) },
    MODE_STORAGE_X2 = const { offset_of!(ModeStorage, x2) },
    MODE_STORAGE_X3 = const { offset_of!(ModeStorage, x3) },
    MODE_STORAGE_X4 = const { offset_of!(ModeStorage, x4) },
    MODE_STORAGE_X5 = const { offset_of!(ModeStorage, x5) },
    MODE_STORAGE_X6 = const { offset_of!(ModeStorage, x6) },
    MODE_STORAGE_X7 = const { offset_of!(ModeStorage, x7) },
    MODE_STORAGE_X8 = const { offset_of!(ModeStorage, x8) },
    MODE_STORAGE_X9 = const { offset_of!(ModeStorage, x9) },
    MODE_STORAGE_X10 = const { offset_of!(ModeStorage, x10) },
    MODE_STORAGE_X11 = const { offset_of!(ModeStorage, x11) },
    MODE_STORAGE_X12 = const { offset_of!(ModeStorage, x12) },
    MODE_STORAGE_X13 = const { offset_of!(ModeStorage, x13) },
    MODE_STORAGE_X14 = const { offset_of!(ModeStorage, x14) },
    MODE_STORAGE_X15 = const { offset_of!(ModeStorage, x15) },
    MODE_STORAGE_X16 = const { offset_of!(ModeStorage, x16) },
    MODE_STORAGE_X17 = const { offset_of!(ModeStorage, x17) },
    MODE_STORAGE_X18 = const { offset_of!(ModeStorage, x18) },
    MODE_STORAGE_X19 = const { offset_of!(ModeStorage, x19) },
    MODE_STORAGE_X20 = const { offset_of!(ModeStorage, x20) },
    MODE_STORAGE_X21 = const { offset_of!(ModeStorage, x21) },
    MODE_STORAGE_X22 = const { offset_of!(ModeStorage, x22) },
    MODE_STORAGE_X23 = const { offset_of!(ModeStorage, x23) },
    MODE_STORAGE_X24 = const { offset_of!(ModeStorage, x24) },
    MODE_STORAGE_X25 = const { offset_of!(ModeStorage, x25) },
    MODE_STORAGE_X26 = const { offset_of!(ModeStorage, x26) },
    MODE_STORAGE_X27 = const { offset_of!(ModeStorage, x27) },
    MODE_STORAGE_X28 = const { offset_of!(ModeStorage, x28) },
    MODE_STORAGE_X29 = const { offset_of!(ModeStorage, x29) },
    MODE_STORAGE_X30 = const { offset_of!(ModeStorage, x30) },

    MODE_STORAGE_SP = const { offset_of!(ModeStorage, sp) },

    MODE_STORAGE_TCR_ELX = const { offset_of!(ModeStorage, tcr_elx) },
    MODE_STORAGE_TTBR0_ELX = const { offset_of!(ModeStorage, ttbr0_elx) },
    MODE_STORAGE_TTBR1_ELX = const { offset_of!(ModeStorage, ttbr1_elx) },
    MODE_STORAGE_MAIR_ELX = const { offset_of!(ModeStorage, mair_elx) },
    MODE_STORAGE_SCTLR_ELX = const { offset_of!(ModeStorage, sctlr_elx) },
    MODE_STORAGE_VBAR_ELX = const { offset_of!(ModeStorage, vbar_elx) },
    MODE_STORAGE_ELR_ELX = const { offset_of!(ModeStorage, elr_elx) },
    MODE_STORAGE_SPSR_ELX = const { offset_of!(ModeStorage, spsr_elx) },
}
