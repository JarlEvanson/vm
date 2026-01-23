ENTRY(revm_entry)

PHDRS {
    code PT_LOAD        FLAGS (0x4 | 0x1);
    rodata PT_LOAD      FLAGS (0x4      );
    relro PT_LOAD  FLAGS (0x4 | 0x2);
    data PT_LOAD        FLAGS (0x4 | 0x2);
    dyn PT_DYNAMIC      FLAGS (0x4      );
}

ACTIVE_PAGE_SIZE = 4096;

SECTIONS {
    . = 0;
    _image_start = .;

    . = ALIGN(ACTIVE_PAGE_SIZE);
    .text : {
        *(.text .text.*)
    } :code

    . = ALIGN(ACTIVE_PAGE_SIZE);
    .rodata : {
        *(.rodata .rodata.*)
    } :rodata

    .dynamic : {
        *(.dynamic .dynamic.*)
    } :dyn :rodata

    . = ALIGN(ACTIVE_PAGE_SIZE);
    .data.rel.ro : {
        *(.data.rel.ro .data.rel.ro.*)
    } :relro

    . = ALIGN(ACTIVE_PAGE_SIZE);
    .data : {
        *(.data .data.*)
    } :data

    .bss : {
        *(.bss .bss.*)
    } :data

    _image_end = .;
}
