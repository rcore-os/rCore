    .section .text,"ax",%progbits
    .globl kern_entry
kern_entry:
    # la sp, bootstacktop
    auipc sp, %hi(bootstacktop)
    addi sp, sp, %lo(bootstacktop)

    # tail rust_main
    auipc t1, %hi(rust_main)
    jalr zero, t1, %lo(rust_main)

.section .data
    .align 12  #PGSHIFT
    .global bootstack
bootstack:
    .space 4096 * 8  #KSTACKSIZE
    .global bootstacktop
bootstacktop:
