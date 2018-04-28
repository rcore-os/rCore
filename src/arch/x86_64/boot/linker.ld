ENTRY(start)

OTHER_OFFSET = 0x7000;
BOOT_OFFSET = 0x100000;
KERNEL_OFFSET = 0xffffff0000000000;

SECTIONS {

  /* bootloader for first processor */

  . = BOOT_OFFSET;

  .rodata.32 :
  {
    /* ensure that the multiboot header is at the beginning */
    KEEP(*(.multiboot_header))
    */boot.o (.rodata)
    . = ALIGN(4K);
  }
  .text.32 :
  {
    */boot.o (.text)
    */long_mode_init.o (.text)
    . = ALIGN(4K);
  }
  .bss.32 :
  {
    */boot.o (.bss)
    . = ALIGN(4K);
  }

  /* bootloader for other processors */

  entryother_start = .;         /* backup va */
  . = OTHER_OFFSET;
  .text.other : AT(entryother_start)
  {
    KEEP(*/entryother.o (.text))
    . = ALIGN(4K);
  }
  entryother_end = . - OTHER_OFFSET + entryother_start;
  . = entryother_end;           /* recover va */

  /* kernel */

  . += KERNEL_OFFSET;

  .user ALIGN(4K): AT(ADDR(.user) - KERNEL_OFFSET)
  {
    KEEP(user/*.o (.data))
    . = ALIGN(4K);
  }

  .rodata : AT(ADDR(.rodata) - KERNEL_OFFSET)
  {
    *(.rodata .rodata.*)
    . = ALIGN(4K);
  }

  .text : AT(ADDR(.text) - KERNEL_OFFSET)
  {
    *(.text .text.*)
    . = ALIGN(4K);
  }

  .data : AT(ADDR(.data) - KERNEL_OFFSET)
  {
    *(.data .data.*)
    . = ALIGN(4K);
  }

  .bss ALIGN(4K): AT(ADDR(.bss) - KERNEL_OFFSET)
  {
    *(.bss .bss.*)
    . = ALIGN(4K);
  }

  .got ALIGN(4K): AT(ADDR(.got) - KERNEL_OFFSET)
  {
    *(.got)
    . = ALIGN(4K);
  }

  .got.plt : AT(ADDR(.got.plt) - KERNEL_OFFSET)
  {
    *(.got.plt)
    . = ALIGN(4K);
  }

  .data.rel.ro : AT(ADDR(.data.rel.ro) - KERNEL_OFFSET) 
  {
    *(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
    . = ALIGN(4K);
  }

  .gcc_except_table : AT(ADDR(.gcc_except_table) - KERNEL_OFFSET)
  {
    *(.gcc_except_table)
    . = ALIGN(4K);
  }
}
