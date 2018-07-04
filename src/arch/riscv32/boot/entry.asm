    .section .text,"ax",%progbits
    .globl kern_entry
kern_entry:
    la sp, bootstacktop
    tail rust_main

.section .data
    .align 12  #PGSHIFT
    .global bootstack
bootstack:
    .space 4096 * 8  #KSTACKSIZE
    .global bootstacktop
bootstacktop:
