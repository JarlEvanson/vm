//! Support for booting using the Linux x86 boot protocol.

use core::arch::global_asm;

global_asm! {
    ".pushsection .linux-efi-header, \"ax\"",

    ".equ section_count_offset, 4 + 2",
    ".equ optional_header_size_offset, 4 + 16",
    ".equ entry_point_offset, 4 + 20 + 16",
    ".equ image_size_offset, 4 + 20 + 56",

    ".equ file_size_offset, 16",
    ".equ file_offset_offset, 20",
    ".equ section_header_size, 40",

    ".equ ext_cmd_line_ptr_offset, 0x0C8",
    ".equ e820_table_size_offset, 0x1e8",
    ".equ base_cmd_line_ptr_offset, 0x228",
    ".equ setup_data_offset, 0x250",
    ".equ e820_table_offset, 0x2D0",
    ".equ boot_params_size, 0xD00 + 0x1EC",

    ".equ command_line_size, 0xFFF",

    ".equ setup_data_next_offset, 0",
    ".equ setup_data_type_offset, 8",
    ".equ setup_data_len_offset, 12",
    ".equ setup_data_data_offset, 16",
    ".equ setup_data_size, 16",

    ".equ setup_indirect_type_offset, 0",
    ".equ setup_indirect_len_offset, 0",
    ".equ setup_indirect_addr_offset, 0",

    ".equ pml3_table_count, 4",
    ".equ pml2_table_count, 17 + 2 + 2",

    "_section_start:",
    ".skip 0x1F1",

    ".byte _real_sectors",      // setup_sects
    ".2byte 0",                 // root_flags
    ".4byte 0x01000000",        // syssize
    ".2byte 0",                 // ram_size
    ".2byte 0",                 // vid_mode
    ".2byte 0",                 // root_dev
    ".2byte 0xAA55",            // boot_flag

    ".global _real_start",
    "_real_start:",

    ".byte 0xEB, 0x6a",         // jump
    ".ascii \"HdrS\"",          // header
    ".2byte 0x020f",            // version
    ".4byte 0",                 // realmode_swtch
    ".2byte 0",                 // start_sys_seg
    ".2byte 0",                 // kernel_version
    ".byte 0",                  // type_of_loader
    ".byte 0x1",                // loadflags
    ".2byte 0",                 // setup_move_size
    ".4byte 0x100000",          // code32_start
    ".4byte 0",                 // ramdisk_image
    ".4byte 0",                 // ramdisk_size
    ".4byte 0",                 // bootsect_kludge
    ".2byte 0",                 // heap_end_ptr
    ".byte 0",                  // ext_loader_ver
    ".byte 0",                  // ext_loader_type
    ".4byte 0",                 // cmd_line_ptr
    ".4byte 0xAFFFFFFF",        // initrd_addr_max
    ".4byte 0x200000",          // kernel_alignment
    ".byte 1",                  // relocatable_kernel
    ".byte 21",                 // min_alignment
    ".2byte 0x3",               // xloadflags
    ".4byte command_line_size", // cmdline_size
    ".4byte 0",                 // hardware_subarch
    ".8byte 0",                 // hardware_subarch_data
    ".4byte 0",                 // payload_offset
    ".4byte 0",                 // payload_length
    ".8byte 0",                 // setup_data
    ".8byte 0x100000",          // pref_address
    ".4byte 0x200000",          // init_size
    ".4byte 0",                 // handover_offset
    ".4byte _kernel_info - _section_start", // kernel_info_offset

    // This is the target of the jump located at the start of `code16`.
    ".byte 0xEB, 0xFE", // Spin forever; we don't support the 16-bit entry point.

    ".align 4096",
    ".global _real_end",
    "_real_end:",

    // 32-bit boot protocol entry point.
    "entry_32:",

    ".code32", // Force code to be interpreted as 32-bit.
    "jmp entry_32",

    // Force `entry_64` to be located precisely 0x200 after `entry_32`.
    "8:", ".space 512 - (8b - entry_32)",

    // 64-bit boot protocol entry point.
    "entry_64:",

    ".code64", // Force code to be interpreted as 64-bit.
    "cld", // Clear direction flag.
    "mov [rip + boot_params_pointer], rsi", // Save RSI (boot_params).

    // We need to map the file data that we care about, the boot_params structure, and provide a
    // slot for temporary mappings using our own page tables in order to allocate without possibly
    // overwriting the data.
    //
    // Next, we need to iterate through the e820 map to build a memory map, then iterate through
    // again in order to mark reserved regions (the stub file, the boot_params structure, the
    // command line, and any setup_data structures).
    //
    // We can then use our memory map to allocate enough pages to load the stub file using the PE
    // file format and provide access to a temporary mapping. This allows us to transition to Rust
    // code.

    // First, we calcuate maximum offset of the file we need to map.

    // Calculate location of section headers.
    "xor rsi, rsi", // Clear upper bits of rsi.
    "mov si, [rip + _pe_header + optional_header_size_offset]", // Load optional header size.
    "add rsi, 4 + 20", // Add PE signature and PE file header sizes.

    // Load section count and calculate total size of section headers.
    "xor rcx, rcx", // Clear upper bits of rcx.
    "mov cx, [rip + _pe_header + section_count_offset]", // Load section count.
    "mov rax, section_header_size", // Load size of each section header (constant).
    "mul rcx", // Calculate total size of section header.

    // Caculate maximum offset of PE header data we care about.
    "mov rbx, rsi",
    "add rbx, rax",

    // Currently, we have the maximum file offset we care about in rbx, the section count in rcx,
    // and the offset of the section headers from the start of the PE header in rsi.
    "lea rdx, [rip + _pe_header]", // Calculate address of PE header.
    "add rdx, rsi", // Calculate the address of the section header table.

    ".pe_header_loop:",
    "mov eax, [rdx + file_offset_offset]", // Load offset of the section data in the file.
    "add eax, [rdx + file_size_offset]", // Add size of the section data in the file.
    "sub eax, 0x400", // Subtract the truncated data from the offset.

    "cmp rax, rbx", // Compare the maximum secton data offset to the maximum relevant file offset.
    "cmovg rbx, rax", // Update the maximum relevant file offset to the larger of the two.

    "add rbx, section_header_size", // Set forward to the next section header.
    "loop .pe_header_loop", // Loop if any more section headers remain.

    // Store the start of the file and the maximum relevant offset of the file.
    "lea rax, [rip + entry_32]",
    "mov [rip + file_pointer], rax",
    "mov [rip + file_size], rbx",
    // Map the currently active file range.
    "call identity_map_region",

    "mov rax, [rip + boot_params_pointer]",
    "mov rbx, boot_params_size",
    // Map the boot parameters page.
    "call identity_map_region",

    "lea rax, [rip + pml4]",
    "mov cr3, rax",

    // Acquire the size of the image when loaded.
    "mov r15d, [rip + _pe_header + image_size_offset]",
    "mov r14, 0",

    "lea rax, [rip + memblock_storage]",
    "mov [rip + memblock_pointer], rax",

    /*
    "outer_loop:",

    "mov rax, r14", // Load e820 current state.
    "call e820_iter", // Get next e820 entry.
    "mov r14, rax", // Store e820 state for later.

    "cmp rdx, 1", // Check that the type of the region is USABLE.
    "jne outer_loop", // Try next entry if not a usable region.

    // Shift values for easier testing.
    "mov rax, rbx",
    "mov rbx, rcx",

    "mov rcx, [rip + file_pointer]",
    "mov rdx, [rip + file_size]",
    "call truncate_range",

    "mov rcx, [rip + boot_params_pointer]",
    "mov rdx, boot_params_size",
    "call truncate_range",

    // TODO: Don't truncate command line or `setup_data`.

    "mov rdx, 4096 - 1", // Calculate mask.
    "mov rcx, rax", // Copy base address.
    "neg rcx", // Negate the base address.
    "and rcx, rdx", // Mask the base address.
    "add rax, rcx", // Calculate the aligned address.
    "sub rbx, rcx", // Reduce the length by the alignment size.

    "cmp rbx, r15",
    "jl outer_loop",
    */
    "mov rax, [rip + memblock_len]",

    "5:", "jmp 5b",

    // Identity maps the given region.
    //
    // # Arguments
    //
    // rax: base address of region.
    // rbx: length of the region.
    "identity_map_region:",
    "push rax",
    "push rbx",

    // Don't allocate for zero-sized mapping.
    "cmp rbx, 0",
    "je .exit",

    // Convert length into end of region.
    "add rbx, rax",

    // Loop until all regions have been processed.
    ".loop:",
    "call identity_map_2mib",
    "add rax, 0x200000",
    "cmp rax, rbx",
    "jl .loop",

    ".exit:",
    "pop rbx",
    "pop rax",
    "ret",

    // Identity maps the 2mib region inside which `rax` resides.
    //
    // # Arguments
    //
    // rax: base address of the region
    "identity_map_2mib:",

    ".equ PML4E_BITS, (1 << 1) | 1",
    ".equ PML3E_BITS, (1 << 1) | 1",
    ".equ PML2E_BITS, (1 << 7) | (1 << 1) | 1",

    ".equ PML_ADDR_MASK, ((1 << 40) - 1) << 12",
    ".equ PML2_HUGE_MASK, ((1 << 31) - 1) << 21",

    "push rax",
    "push rbx",
    "push rcx",
    "push rdx",
    "push rsi",

    // Calculate pml4e index.
    "mov rcx, rax",
    "shr rcx, 39",
    "and rcx, 0x1FF",

    // Load address of pml4 table.
    "lea rbx, [rip + pml4]",

    // Load pml4 table entry and check if it has been allocated already.
    "mov rdx, [rbx + 8 * rcx]",
    "cmp rdx, 0",
    "jne .pml4e_mapped",

    "lea rsi, [rip + pml3]", // Load base of embedded pml3 tables.
    "xor rdx, rdx",
    "mov dl, [rip + pml3_next_index]", // Load next index of pml3 table.
    "cmp rdx, pml3_table_count",
    "jge fail64",
    "inc byte ptr [rip + pml3_next_index]", // Increment the index for next time.
    "shl rdx, 12", // Multiply index by 4096.
    "add rsi, rdx", // Calculate base of pml3 table to use.

    "mov rdx, PML_ADDR_MASK",
    "and rdx, rsi",
    "or rdx, PML4E_BITS",

    "mov [rbx + 8 * rcx], rdx",

    // rdx holds value of table entry.
    ".pml4e_mapped:",
    // Calculate pml3e index.
    "mov rcx, rax",
    "shr rcx, 30",
    "and rcx, 0x1FF",

    // Calculate address of pml3 table.
    "mov rbx, PML_ADDR_MASK",
    "and rbx, rdx",

    // Load the pml3 table entry and check if it has been allocated already.
    "mov rdx, [rbx + 8 * rcx]",
    "cmp rdx, 0",
    "jne .pml3e_mapped",

    "lea rsi, [rip + pml2]", // Load base of embedded pml2 tables.
    "xor rdx, rdx",
    "mov dl, [rip + pml2_next_index]", // Load next index of pml2 table.
    "cmp rdx, pml2_table_count",
    "jge fail64",
    "inc byte ptr [rip + pml2_next_index]", // Increment table index for latter use.
    "shl rdx, 12", // Multiply index by 4096.
    "add rsi, rdx", // Calculate base of pml2 table to use.

    "mov rdx, PML_ADDR_MASK",
    "and rdx, rsi",
    "or rdx, PML3E_BITS",

    "mov [rbx + 8 * rcx], rdx",

    ".pml3e_mapped:",
    // Calculate pml2e index.
    "mov rcx, rax",
    "shr rcx, 21",
    "and rcx, 0x1FF",

    // Calculate address of pml2 table.
    "mov rbx, PML_ADDR_MASK",
    "and rbx, rdx",

    "mov rdx, PML2_HUGE_MASK",
    "and rdx, rax",
    "or rdx, PML2E_BITS",

    "mov [rbx + 8 * rcx], rdx",

    "pop rsi",
    "pop rdx",
    "pop rcx",
    "pop rbx",
    "pop rax",

    "ret",

    // Temporarily maps the region inside which `rax` resides.
    //
    // # Arguments
    //
    // rax: base address of the region
    // rbx: length of the region (must be less than or equal to 2 MiB).
    //
    // # Returns
    //
    // # rax: base address converted to the temporary mapping location
    "map_tmp:",

    "push rbx",
    "push rcx",
    "push rdx",

    "push rax",

    "lea rbx, [rip + pml4]", // Load address of pml4 table.

    "lea rcx, [rip + tmp_pml3]", // Load base of embedded temporary pml3 table.

    "mov rdx, PML_ADDR_MASK",
    "and rdx, rcx",
    "or rdx, PML4E_BITS",

    "mov [rbx + 8 * 256], rdx", // Store the address of the temporary pml3 table.

    "mov rbx, rcx",

    "lea rcx, [rip + tmp_pml2]",

    "mov rdx, PML_ADDR_MASK",
    "and rdx, rcx",
    "or rdx, PML3E_BITS",

    "mov [rbx], rdx", // Store the address of the temporary pml2 table.

    "mov rbx, rcx",

    "mov rcx, PML2_HUGE_MASK",
    "and rcx, rax",
    "or rcx, PML2E_BITS",

    "add rax, 0x200000",
    "mov rdx, PML2_HUGE_MASK",
    "and rdx, rax",
    "or rdx, PML2E_BITS",

    "mov [rbx], rcx",
    "mov [rbx + 8], rdx",

    "pop rax",
    "mov rdx, 0x1FFFFF",
    "and rax, rdx",
    "mov rbx, 0xFFFF800000000000",
    "add rax, rbx",

    "pop rdx",
    "pop rcx",
    "pop rbx",

    "ret",

    "reload_cr3:",

    "push rax",

    "mov rax, cr3",
    "mov cr3, rax",

    "pop rax",

    "ret",

    // Returns the next e820 entry in the boot parameters. This function uses temporary mappings.
    //
    // # Arguments:
    //
    // rax: The value zero (to start the chain) or the state value returned from previous
    // invocations.
    //
    // # Return Values
    //
    // rax: The state value that can be used in the next call to `e820` iter. If the state value
    // returned is zero, then the values in rbx, rcx, and edx are not valid and all entries have
    // been outputed.
    // rbx: The base address of the e820 entry.
    // rcx: The length of the e820 entry.
    // edx: The type of the e820 entry.
    "e820_iter:",

    ".equ e820_address, 0",
    ".equ e820_size, 8",
    ".equ e820_type, 16",

    "cmp rax, 0", // Check if rax is zero (iter is starting).
    "jne .body", // Jump to body if iter state is initialized.

    "mov rax, [rip + boot_params_pointer]", // Load address of boot_params.
    "mov rbx, [rax + e820_table_size_offset]",
    "cmp rbx, 0",
    "je .e820_table_empty",

    "lea rax, [rax + e820_table_offset]", // Load base address of e820 table.
    "jmp .body",

    ".e820_table_empty:",
    "mov rax, [rax + setup_data_offset]", // Load the address of the first `setup_data` header.

    ".body:",

    "mov rbx, [rip + boot_params_pointer]",
    "mov rdx, [rbx + e820_table_size_offset]", // Get size of e820 table.
    "mov rcx, rdx", // Copy size.
    "shl rcx, 2", // Multiply size by 4.
    "add rdx, rcx", // rdx = rdx * 5
    "shl rdx, 2", // rdx = rdx * 5 (original rdx * 20). Size of e820 table calculated.
    "lea rcx, [rbx + e820_table_offset]", // Get base of e820 table.
    "cmp rax, rcx", // Check if `rax` is less than e820 table.
    "jl .setup_data",
    "add rcx, rdx", // Calculate top of table.
    "cmp rax, rcx", // Check if `rax` is greater than e820 table.
    "jg .setup_data",

    // `rax` is inside the table.
    "push rax",

    "add rax, 20", // Calculate address of next table entry.
    "cmp rax, rcx", // Check that the next table entry is valid.
    "jl .e820_table", // If so, continue to load data into registers.

    "mov rax, [rbx + setup_data_offset]", // Start with `setup_data` header data.
    "mov rcx, (1 << 63)",
    "or rax, rcx",

    ".e820_table:",
    "xchg rax, [rsp]", // Swap current state and next state.

    "mov rbx, [rax + e820_address]",
    "mov rcx, [rax + e820_size]",
    "mov edx, [rax + e820_type]",

    "pop rax",
    "ret",

    // `rax` is in `setup_data` structures.
    ".setup_data:",

    "mov rbx, (1 << 63) - 1",
    "and rax, rbx",
    "cmp rax, 0",
    "je .end",

    // We have a valid `setup_data` header pointed to be `rax`.
    //
    // Map it.
    "mov rbx, setup_data_size",
    "call map_tmp",

    "mov rdx, [rax + setup_data_type_offset]", // Load setup_data type.
    "cmp rdx, 1", // Check if the type is E820_EXT.
    "jne .check_indirect", // If it isn't, go to the next check.

    "mov rbx, [rax + setup_data_data_offset + e820_address]",
    "mov rcx, [rax + setup_data_data_offset + e820_size]",
    "mov edx, [rax + setup_data_data_offset + e820_type]",

    "mov rax, [rax + setup_data_next_offset]",

    "push rbx",
    "mov rbx, (1 << 63)",
    "or rax, rbx",
    "pop rbx",

    "ret",

    ".check_indirect:",
    "mov rbx, 1 << 31",
    "cmp rdx, rbx", // Check if the type is SETUP_INDIRECT.
    "jne .fallback", // If it isn't, go to the next check.
    "mov rdx, [rax + setup_data_data_offset + setup_indirect_type_offset]",
    "or rbx, 1",
    "cmp rdx, rbx",
    "jne .fallback",

    "mov rbx, [rax + setup_data_data_offset + setup_indirect_addr_offset]",

    "mov rcx, [rbx + e820_size]",
    "mov edx, [rbx + e820_type]",
    "mov rbx, [rbx + e820_address]",

    "push rbx",
    "mov rbx, (1 << 63)",
    "or rax, rbx",
    "pop rbx",

    "ret",

    ".fallback:",
    "mov rax, [rax + setup_data_next_offset]", // Load next pointer.
    "mov rbx, (1 << 63)",
    "or rax, rbx", // Prevent it from being recognized as zero by state.
    "jmp .setup_data", // Restart this function.

    ".end:",
    "mov rax, 0",
    "ret",


    // Adds a free region to an allocator.
    //
    // # Arguments
    //
    // rax: The base of the region to add.
    // rbx: The length of the reigon to add.
    //
    // # Returns
    //
    // The carry flag will be set if the memblock is full.
    "memblock_add:",
    "call memblock_add_no_merge",
    "call memblock_merge",
    "ret",

    // Adds a free region to an allocator without merging any blocks.
    //
    // # Arguments
    //
    // rax: The base of the region to add.
    // rbx: The length of the reigon to add.
    //
    // # Returns
    //
    // The carry flag will be set if the memblock is full.
    "memblock_add_no_merge:",

    "push rcx",
    "push rdx",
    "push rdi",
    "push rsi",

    "mov rcx, [rip + memblock_len]",
    "mov rdx, [rip + memblock_size]",
    "cmp rcx, rdx", // Check if the memblock array is full.
    "jge .full", // Jump to full-handling.

    "mov rcx, 0",
    "mov rsi, [rip + memblock_pointer]",

    "memblock_add.find_loop:",

    // Check if we've reached the end of the array.
    "cmp rcx, [rip + memblock_len]",
    "jge memblock_add.found",

    "mov rdx, [rsi]",
    "cmp rax, rdx",
    "jl memblock_add.found",

    "inc rcx",
    "add rsi, 16",
    "jmp memblock_add.find_loop",

    "memblock_add.found:",

    "mov rdx, [rip + memblock_len]",
    "sub rdx, rcx",
    "shl rdx, 4",

    "cmp rdx, 0",
    "je 5f",

    "push rcx",
    "push rsi",

    "mov rcx, rdx",
    "mov rdi, rsi",
    "add rdi, 16",
    "rep movsb",

    "pop rsi",
    "pop rcx",

    "5:",

    "mov [rsi], rax",
    "mov [rsi + 8], rbx",

    "inc qword ptr [rip + memblock_len]",

    "clc",
    "jmp memblock.exit",

    ".full:",
    "stc",

    "memblock.exit:",

    "pop rsi",
    "pop rdi",
    "pop rdx",
    "pop rcx",

    "ret",

    "memblock_merge:",

    "push rax",
    "push rcx",
    "push rdx",
    "push rsi",
    "push rdi",
    "push rbp",

    "mov rbp, [rip + memblock_pointer]",
    "mov rdx, [rip + memblock_len]",

    // Calculate end of memblock array.
    "shl rdx, 4",
    "add rdx, rbp",

    // Iterate over all entries in the memblock map.
    //
    // rdx: pointer indicating the end of the memblock array.
    // rbp: pointer to the entry we are attempting to merge with its successors.
    "memblock_merge.outer_loop:",

    // If the loop has reached its end, then exit.
    "cmp rbp, rdx",
    "jge memblock_merge.outer_loop_exit",

    "mov rax, [rbp]", // Load memblock entry address.
    "add rax, [rbp + 8]", // Calculate the end of the region described by the memblock entry.

    "lea rsi, [rbp + 16]", // Calculate the address of the next entry in the memblock array.

    "mov rcx, 0",

    // Iterate over the successors until a successor cannot be merged.
    //
    // rax: the physical address at the end of `rbp`'s entry.
    // rcx: the number of successfully merged entries in the inner loop.
    // rdx: pointer indicating the end of the memblock array.
    // rbp: pointer to the entry we are attempting to merge with its successors.
    // rsi: pointer to the successor entry we are testing for mergeability.
    "memblock_merge.inner_loop:",

    "cmp rsi, rdx",
    "jge memblock_merge.inner_loop_exit",

    "cmp rax, [rsi]",
    "jl memblock_merge.inner_loop_exit",

    "add rax, [rsi + 8]",
    "inc rcx",
    "add rsi, 16",
    "jmp memblock_merge.inner_loop",

    // The inner loop has finished: we have discovered the limit of what can be merged into the
    // single entry.
    //
    // We must update the entry and copy all entries forward.
    //
    // rax: the physical address at the end of the newly merged region.
    // rcx: the number of successfully merged entries in the inner loop.
    // rdx: pointer indicating the end of the memblock array.
    // rbp: pointer to the entry we are attempting to merge with its successors.
    // rsi: pointer to the successor entry we are testing for mergeability.
    "memblock_merge.inner_loop_exit:",

    "sub [rip + memblock_len], rcx",
    "shl rcx, 4",
    "sub rdx, rcx",

    "sub rax, [rbp]",
    "mov [rbp + 8], rax",

    "lea rdi, [rbp + 16]",
    "mov rcx, rdx",
    "sub rcx, rdi",

    "rep movsb",

    "add rbp, 16",
    "jmp memblock_merge.outer_loop",

    // The outer loop has finished: we have merged all possible entries.
    //
    // rdx: pointer to the end of the memblock array.
    "memblock_merge.outer_loop_exit:",

    "pop rbp",
    "pop rdi",
    "pop rsi",
    "pop rdx",
    "pop rcx",
    "pop rax",

    "ret",

    // Removes a region from the memblock allocator.
    //
    // # Arguments
    //
    // rax: the base of the region to remove.
    // rbx: the length of the region to remove.
    //
    // # Returns
    //
    // The carry flag will be set if a region must be split and the memblock array is not large
    // enough to handle such splitting.
    "memblock_remove:",

    "call memblock_merge",

    "mov rdx, 0", // Load starting index.
    "add rbx, rax", // Calculate end address of region to be removed.

    "memblock_remove.loop:",

    // Check if the index is beyond the array.
    "cmp rdx, [rip + memblock_len]",
    "je memblock_remove.exit_success",

    // Caclulate address of index to be compared.
    "mov rsi, [rip + memblock_pointer]",
    "lea rsi, [rsi + rdx * 8]",
    "lea rsi, [rsi + rdx * 8]",

    // Calculate end address of region to be tested for removal.
    "mov rcx, [rsi]",
    "add rcx, [rsi + 8]",

    // First, handle test region being completely contained within removal region.
    "cmp rax, [rsi]",
    "jg 5f",

    "cmp rbx, rcx",
    "jl 5f",

    // Check for complete containment succeeded.
    "mov rdi, rsi",
    "add rsi, 16",
    "mov rcx, [rip + memblock_len]",
    "sub rcx, rdx",
    "shl rcx, 4",

    "rep movsb",

    "dec qword ptr [rip + memblock_len]",
    "jmp memblock_remove.loop",

    // Test region is not completely contained within removal region.
    //
    // Next, handle test region needing truncated on the lower side.
    "5:",

    "cmp rbx, [rsi]",
    "jle 5f",

    "cmp rbx, rcx",
    "jge 5f",

    "sub rcx, rbx", // Test region end - removal region end = length of truncated region.
    "mov [rsi], rbx", // Store start of truncated region.
    "mov [rsi + 8], rcx", // Store length of truncated region.

    "jmp memblock_remove.loop",

    // Test region does not need truncated on the lower side.
    //
    // Next, handle test region needing truncated on the upper side.
    "5:",

    "cmp rax, [rsi]",
    "jge 5f",

    "cmp rax, rcx",
    "jle 5f",

    "mov rcx, rax",
    "sub rcx, [rsi]", // Removal region start - test region start = length of truncated region.
    "mov [rsi + 8], rcx", // Store length of truncated region.

    "jmp memblock_remove.loop",

    // Test region does not need truncated on upper side.
    //
    // Finally, handle region that must be split.
    "5:",

    "cmp rax, [rsi]",
    "jle 5f",

    "cmp rbx, rcx",
    "jge 5f",

    // Check that memblock array has enough room to split the entry.
    "mov rsi, [rip + memblock_size]",
    "cmp [rip + memblock_len], rsi",
    "jge memblock_remove.exit_failure",

    "mov rcx, rax",
    "sub rcx, [rsi]", // Removal region start - test region start = length of lower region.
    "xchg rcx, [rsi + 8]", // Exchange lower region length and test region length.

    "add rcx, [rsi]", // End of test region.
    "sub rcx, rbx", // End of test region - end of removal region = length of upper region.

    

    "jmp memblock_remove.loop",

    "5:",

    "inc rdx",
    "jmp memblock_remove.loop",

    "memblock_remove.exit_failure:",

    "stc",
    "jmp 5f",

    "memblock_remove.exit_success:",
    "clc",

    "5:",

    "ret",

    "fail64:", "hlt", "jmp fail64",
    /*
    // We need to map the file data that we care about and the boot_params structure using our own
    // page tables in order to be able to allocate.




    // We need to find a region of memory that is large enough to contain the loaded PE file and
    // does not overlap with our loaded blob, the zero page, the command line buffer, or the
    // setup_data.

    // Store the total size of the loaded PE file in `r15`.
    "mov r15d, [rip + _pe_header + image_size_offset]",

    // Next, calculate the maximum offset of our loaded blob that we care about.

    // First, we acquire the number of sections we have.
    "xor rax, rax",
    "mov ax, [rip + _pe_header + section_count_offset]", // Load section count.

    "xor r13, r13",
    "mov r13w, [rip + _pe_header + optional_header_size_offset]", // Load optional header size.
    "add r13w, 4 + 20", // Add PE signature and PE file header sizes.

    "mov rbx, section_header_size",
    "mul rbx", // Total size of section headers.
    "add rax, r13",
    "mov r14, rax", // `r14` stores the current maximum file offset we care about.

    "xor rcx, rcx",
    "mov cx, [rip + _pe_header + section_count_offset]", // Initialize iteration count.
    "lea rbx, [rip + _pe_header]",
    "add rbx, r13", // Initialize section header table offset.

    ".pe_header_loop:",

    "mov eax, [rbx + file_size_offset]",
    "add eax, [rbx + file_offset_offset]",

    "cmp rax, r14",
    "cmovg r14, rax",

    "add rbx, section_header_size",
    "loop .pe_header_loop",

    // We need to find a region of physical memory that is `image_size` bytes long
    // and does not overlap with our file, the zero page, the command line buffer, or the
    // setup_data".

    // Store the zero page range. Offset is already stored.
    "mov [rip + boot_params_size_store], boot_params_size",

    // Store the file range.
    "lea rax, [rip + entry_32]",
    "mov [rip + file_pointer], rax",
    "mov [rip + file_size], r14",

    // Store the setup_data header pointer.
    "mov ebx, [rax + setup_data_offset]",
    "mov [rip + setup_data_header_pointer], rbx",

    //


    // Store the command line range.
    "mov rax, [rip + boot_params_pointer]",
    "mov ebx, [rax + base_cmd_line_ptr_offset]",
    "mov ecx, [rax + ext_cmd_line_ptr_offset]",
    "shl rcx, 32",
    "add rbx, rcx",
    "mov [rip + cmd_line_pointer], rbx",
    "mov [rip + cmd_line_size], command_line_size",
    
    "5:", "jmp 5b",
    */


    ".align 4",
    "_kernel_info:",
    ".ascii \"LTop\"",
    ".4byte _kernel_info_var_len_data - _kernel_info",
    ".4byte _kernel_info_end - _kernel_info",
    ".4byte 0x01234567",

    "_kernel_info_var_len_data:",
    "_kernel_info_end:",

    ".align 8",
    "boot_params_pointer:", ".8byte 0",
    "file_pointer:", ".8byte 0",
    "file_size:", ".8byte 0",
    "cmd_line_pointer:", ".8byte 0",

    ".equ memblock_storage_size, 128",
    "memblock_pointer:", ".8byte 0",
    "memblock_len:", ".8byte 0",
    "memblock_size:", ".8byte memblock_storage_size",

    "pml3_next_index:", ".byte 0",
    "pml2_next_index:", ".byte 0",

    ".align 4096",
    "pml4:", ".space 1 * 4096",  // We need 1 top level page table.
    "pml3:", ".space pml3_table_count * 4096",
    "pml2:", ".space pml2_table_count * 4096",

    "tmp_pml3:", ".space 4096", // We need 1 pml3 temporary table.
    "tmp_pml2:", ".space 4096", // We need 1 pml2 temporary table.

    "memblock_storage:", ".space 16 * memblock_storage_size",

    ".align 8",
    "_pe_header:",

    ".popsection",
}
