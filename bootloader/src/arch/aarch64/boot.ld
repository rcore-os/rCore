ENTRY(_start)

SECTIONS {
  . = 0x80000; /* Raspbery Pi 3 AArch64 (kernel8.img) load address */

  .text : {
    KEEP(*(.text.boot)) /* from boot.S */
    *(.text .text.* .gnu.linkonce.t*)
    . = ALIGN(4K);
  }

  .rodata : {
    *(.rodata .rodata.* .gnu.linkonce.r*)
    . = ALIGN(4K);
  }

  .data : {
    *(.data .data.* .gnu.linkonce.d*)
    . = ALIGN(4K);
  }

  .bss : {
    _sbss = .;
    *(.bss .bss.*)
    *(COMMON)
    . = ALIGN(4K);
    _ebss = .;
  }

  .payload : {
    *(.payload)
  }

  /DISCARD/ : { *(.comment) *(.gnu*) *(.note*) *(.eh_frame*) }
}
