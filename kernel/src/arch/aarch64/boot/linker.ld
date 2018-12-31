ENTRY(_start)

SECTIONS {
  . = 0x80000; /* Raspbery Pi 3 AArch64 (kernel8.img) load address */

  .boot : {
    KEEP(*(.text.boot)) /* from boot.S */
  }

  . = 0x100000; /* Load the kernel at this address. It's also kernel stack top address */
  bootstacktop = .;

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

  .bss (NOLOAD) : {
    . = ALIGN(32);
    sbss = .;
    *(.bss .bss.*)
    *(COMMON)
    . = ALIGN(4K);
    ebss = .;
  }

  /* end of the binary */
  _end = ALIGN(8);

  /* number of bytes in BSS section and complete binary */
  __bss_length = (ebss - sbss);
  __binary_length = (_end - _start);

  /DISCARD/ : { *(.comment) *(.gnu*) *(.note*) *(.eh_frame*) }
}
