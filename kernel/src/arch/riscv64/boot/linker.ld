/* Copy from bbl-ucore : https://ring00.github.io/bbl-ucore      */

/* Simple linker script for the ucore kernel.
   See the GNU ld 'info' manual ("info ld") to learn the syntax. */

OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0x80020000;

SECTIONS
{
    . = 0x80000000;
    .boot : {
        KEEP(*(.text.boot))
    }

    /* Load the kernel at this address: "." means the current address */
    . = BASE_ADDRESS;
    start = .;

    .text : {
        stext = .;
        *(.text.entry)
        *(.text .stub .text.* .gnu.linkonce.t.*)
        . = ALIGN(4K);
        etext = .;
    }

    .rodata : {
        srodata = .;
        *(.rodata .rodata.* .gnu.linkonce.r.*)
        . = ALIGN(4K);
        erodata = .;
    }

    .data : {
        sdata = .;
        *(.data .data.*)
        . = ALIGN(4K);
        edata = .;
    }

    .bss : {
        sbss = .;
        *(.bss .bss.* .sbss*)
        . = ALIGN(4K);
        ebss = .;
    }

    .got : {
        *(.got .got.*)
        . = ALIGN(4K);
    }

    PROVIDE(end = .);
}
