OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x80000000;

SECTIONS
{
    . = BASE_ADDRESS;
    skernel = .;

    /* 1) Entry of M->S (init): "_start" */
    .text.init : ALIGN(4)
    {
        KEEP(*(.text.init))
    }

    /* 2) M-mode Trap Handler: "m_trap_deadloop" */
    .text.trap : ALIGN(4)
    {
        KEEP(*(.text.trap))
    }

    /* 3) Entry of S-mode: "j rust_main"  */
    .text.smode : ALIGN(4)
    {
        KEEP(*(.text.smode))
    }

    /* 4) S-mode Codes */
    stext = .;
    .text.entry : ALIGN(4)
    {
        *(.text.entry)
    }
    .text : ALIGN(4)
    {
        *(.text .text.*)
    }
    etext = .;

    /* 5) rodata */
    . = ALIGN(4K);
    srodata = .;
    .rodata : ALIGN(4)
    {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    erodata = .;

    /* 6) data */
    . = ALIGN(4K);
    sdata = .;
    .data : ALIGN(4)
    {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    edata = .;

    /* 7) bss */
    . = ALIGN(4K);
    sbss = .;
    .bss : ALIGN(4)
    {
        *(.bss.stack)
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }
    ebss = .;

    ekernel = .;

    /DISCARD/ :
    {
        *(.eh_frame)
    }
}