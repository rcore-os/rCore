/* Simple linker script for the ucore kernel.
   See the GNU ld 'info' manual ("info ld") to learn the syntax. */

OUTPUT_ARCH(riscv)
ENTRY(_start)

#ifdef board_thinpad
BASE_ADDRESS = 0x80000000;
#else
BASE_ADDRESS = 0x80100000;
#endif

SECTIONS
{

    . = BASE_ADDRESS;
    start = .;

    .text : {
        stext = .;
        *(.text.entry)
        . = ALIGN(4K);
        *(.text.ebase)
        _copy_user_start = .;
        *(.text.copy_user)
        _copy_user_end = .;
        *(.text .text.*)
        . = ALIGN(4K);
        etext = .;
    }

    .rodata : {
        srodata = .;
        *(.rodata .rodata.*)
        *(.dtb)
        . = ALIGN(4K);
        erodata = .;
    }

    .data : {
        sdata = .;
        *(.data .data.*)
        edata = .;
    }

    .stack : {
        *(.bss.stack)
    }

    .bss : {
        sbss = .;
        *(.bss .bss.*)
        ebss = .;
    }

    PROVIDE(end = .);
}
