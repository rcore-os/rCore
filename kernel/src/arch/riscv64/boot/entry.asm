    .section .text.entry
    .globl _start
_start:
    add t0, a0, 1
    slli t0, t0, 16
    
    lui sp, %hi(bootstack)
    addi sp, sp, %lo(bootstack)
    add sp, sp, t0

    call rust_main

    .section .bss
    .align 12  #PGSHIFT
    .global bootstack
bootstack:
    .space 4096 * 16 * 8
    .global bootstacktop
bootstacktop:
