    .section .text.entry
    .globl _start
_start:
    lui  sp, %hi(bootstacktop)
    addi sp, sp, %lo(bootstacktop)
    call rust_main

    .section .bss
    .align 12  #PGSHIFT
    .global bootstack
bootstack:
    .space 4096 * 16  #KSTACKSIZE
    .global bootstacktop
bootstacktop:
