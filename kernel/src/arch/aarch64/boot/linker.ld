ENTRY(_start)

SECTIONS {
  . = 0xffff000000100000; /* Load the kernel at this address. It's also kernel stack top address */

  .text : {
    stext = .;
    *(.text.entry)
    *(.text .text.* .gnu.linkonce.t*)
    . = ALIGN(4K);
    etext = .;
  }

  .rodata : {
    srodata = .;
    *(.rodata .rodata.* .gnu.linkonce.r*)
    . = ALIGN(4K);
    erodata = .;
  }

  .data : {
    sdata = .;
    *(.data .data.* .gnu.linkonce.d*)
    . = ALIGN(4K);
    edata = .;
  }

  .bss : {
    . = ALIGN(32);
    sbss = .;
    *(.bss .bss.*)
    *(COMMON)
    . = ALIGN(4K);
    ebss = .;
  }

  .stack : {
    . = ALIGN(4K);
    bootstack = .;
    . += 0x100000;
    bootstacktop = .;
  }

  /* end of the binary */
  _end = ALIGN(8);

  /DISCARD/ : { *(.comment) *(.gnu*) *(.note*) *(.eh_frame*) }
}
