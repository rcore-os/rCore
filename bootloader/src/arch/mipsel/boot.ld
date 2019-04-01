ENTRY(_start)

SECTIONS {

  /* MIPS entry point after cold reset */
  . = 0xBFC00000;

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

  .dtb : {
    *(.dtb)
  }

  /DISCARD/ : { *(.comment) *(.gnu*) *(.note*) *(.eh_frame*) }
}
