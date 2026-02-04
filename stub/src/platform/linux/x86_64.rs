//! Support for booting using the Linux x86 boot protocol.

use core::arch::global_asm;

global_asm! {
    ".pushsection .linux-efi-header, \"ax\"",

    // Constants related to parsing the PE file.
    ".equ section_count_offset, 4 + 2",
    ".equ optional_header_size_offset, 4 + 16",
    ".equ entry_point_offset, 4 + 20 + 16",
    ".equ image_size_offset, 4 + 20 + 56",

    ".equ file_size_offset, 16",
    ".equ file_offset_offset, 20",
    ".equ section_header_size, 40",

    // Constants related to the `boot_params` table.
    ".equ ext_cmd_line_ptr_offset, 0x0C8",
    ".equ e820_table_size_offset, 0x1e8",
    ".equ base_cmd_line_ptr_offset, 0x228",
    ".equ setup_data_offset, 0x250",
    ".equ e820_table_offset, 0x2D0",
    ".equ boot_params_size, 0xD00 + 0x1EC",

    // Constants related to the constant space page tables.
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
    ".4byte 0xFFF",             // cmdline_size
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

    // Initialize stack pointer.
    ".equ stack_offset, _stack_top - _section_start",
    "mov esp, 0x100000 + stack_offset",

    "5:",
    "jmp 5b",

    // Force `entry_64` to be located precisely 0x200 after `entry_32`.
    "8:", ".space 512 - (8b - entry_32)",

    // 64-bit boot protocol entry point.
    "entry_64:",

    ".code64", // Force code to be interpreted as 64-bit.

    // Initialize stack pointer.
    "lea rsp, [rip + _stack_top]",

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

    // Map the boot parameters page.
    "mov rax, [rip + boot_params_pointer]",
    "mov rbx, boot_params_size",
    "call identity_map_region",

    // Switch to the constant space page allocator.
    "lea rax, [rip + pml4]",
    "mov cr3, rax",

    "5:",
    "jmp 5b",

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

    // Reloads the CR3 register, thereby flushing the page tables.
    "reload_cr3:",

    "push rax",

    "mov rax, cr3",
    "mov cr3, rax",

    "pop rax",

    "ret",

    "fail64:", "hlt", "jmp fail64",

    "linux_header_end:",

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

    // Constant space page table data.
    "pml3_next_index:", ".byte 0",
    "pml2_next_index:", ".byte 0",

    // Constant space page table storage.
    ".align 4096",
    "pml4:", ".space 1 * 4096",  // We need 1 top level page table.
    "pml3:", ".space pml3_table_count * 4096",
    "pml2:", ".space pml2_table_count * 4096",

    "tmp_pml3:", ".space 4096", // We need 1 pml3 temporary table.
    "tmp_pml2:", ".space 4096", // We need 1 pml2 temporary table.

    ".space 8192",
    "_stack_top:",

    ".align 8",
    "_pe_header:",

    ".popsection",
}
