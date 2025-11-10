ENTRY(main)

PHDRS {
    code PT_LOAD        FLAGS (0x4 | 0x1);
    rodata PT_LOAD      FLAGS (0x4      );
    relro PT_LOAD  FLAGS (0x4 | 0x2);
    data PT_LOAD        FLAGS (0x4 | 0x2);
    blob PT_LOAD        FLAGS (0x4 | 0x2);
    dyn PT_DYNAMIC      FLAGS (0x4      );
}

SECTIONS {
    . = 0;
    _image_start = .;

    .linux-efi-header : {
        KEEP(*(.linux-efi-header .linux-efi-header.*))
    } :NONE

    . = ALIGN(CONSTANT(COMMONPAGESIZE));
    .text : {
        *(.text .text.*)
    } :code

    . = ALIGN(CONSTANT(COMMONPAGESIZE));
    .rodata : {
        *(.rodata .rodata.*)
    } :rodata

    .dynamic : {
        *(.dynamic .dynamic.*)
    } :dyn :rodata

    . = ALIGN(CONSTANT(COMMONPAGESIZE));
    .data.rel.ro : {
        *(.data.rel.ro .data.rel.ro.*)
    } :relro

    .got : {
        _got_start = .;
        *(.got)
        _got_end = .;
        
        _got_plt_start = .;
        *(.got.plt .got.plt.*)
        _got_plt_end = .;
    } :relro

    . = ALIGN(CONSTANT(COMMONPAGESIZE));
    .data : {
        *(.data .data.*)
    } :data

    .limine : {
        *(.limine.start)
        *(.limine .limine.*)
        *(.limine.end)
    } :data

    .bss : {
        *(.bss .bss.*)
    } :data

    . = ALIGN(CONSTANT(COMMONPAGESIZE));
    .blob : {
        _blob_start = .;
    } :blob
}
