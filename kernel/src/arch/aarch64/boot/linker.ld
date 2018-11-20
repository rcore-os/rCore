ENTRY(_start)

SECTIONS {
  . = 0x80000; /* Raspbery Pi 3 Aarch64 (kernel8.img) load address */

  .boot : {
    KEEP(*(.text.boot)) /* from boot.S */
  }

  . = 0x100000; /* Load the kernel at this address. It's also kernel stack top address */

  .text : {
    *(.text.entry)
    *(.text .text.* .gnu.linkonce.t*)
  }

  .rodata : {
    *(.rodata .rodata.* .gnu.linkonce.r*)
  }

  .data : {
    *(.data .data.* .gnu.linkonce.d*)
  }

  .bss (NOLOAD) : {
    . = ALIGN(32);
    __bss_start = .;
    *(.bss .bss.*)
    *(COMMON)
    . = ALIGN(8);
    __bss_end = .;
  }

  /* end of the binary */
  _end = ALIGN(8);

  /* number of bytes in BSS section and complete binary */
  __bss_length = (__bss_end - __bss_start);
  __binary_length = (_end - _start);

  /DISCARD/ : { *(.comment) *(.gnu*) *(.note*) *(.eh_frame*) }
}
