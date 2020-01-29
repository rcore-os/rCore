    .section .text.entry
    .globl _start
_start:
    # a0 == hartid
    # pc == 0x80010000
    # sp == 0x8000xxxx

    # 1. set sp
    # sp = bootstack + (hartid + 1) * 0x10000
    add     t0, a0, 1
    slli    t0, t0, 14
    lui     sp, %hi(bootstack)
    add     sp, sp, t0

    # 1.1 set device tree paddr
    # OpenSBI give me 0 ???
    li      a1, 0x800003b0

    # 2. jump to rust_main (absolute address)
    lui     t0, %hi(rust_main)
    addi    t0, t0, %lo(rust_main)
    jr      t0

    .section .bss.stack
    .align 12   # page align
    .global bootstack
bootstack:
    .space 4096 * 4 * 2
    .global bootstacktop
bootstacktop:
