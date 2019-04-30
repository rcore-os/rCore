    .section .text.entry
    .globl _start
_start:
    # a0 == hartid
    # pc == 0x80200000
    # sp == 0x800xxxxx

    # 1. set sp
    # sp = bootstack + (hartid + 1) * 0x10000
    add     t0, a0, 1
    slli    t0, t0, 14
    lui     sp, %hi(bootstack)
    add     sp, sp, t0

    # 2. enable paging
    # satp = (8 << 60) | PPN(boot_page_table_sv39)
    lui     t0, %hi(boot_page_table_sv39)
    li      t1, 0xffffffffc0000000 - 0x80000000
    sub     t0, t0, t1
    srli    t0, t0, 12
    li      t1, 8 << 60
    or      t0, t0, t1
    csrw    satp, t0
    sfence.vma

    # 3. jump to rust_main (absolute address)
    lui     t0, %hi(rust_main)
    addi    t0, t0, %lo(rust_main)
    jr      t0

    .section .bss.stack
    .align 12   # page align
    .global bootstack
bootstack:
    .space 4096 * 4 * 8
    .global bootstacktop
bootstacktop:

    .section .data
    .align 12   # page align
boot_page_table_sv39:
    # 0x00000000_80000000 -> 0x80000000 (1G)
    # 0xffffffff_c0000000 -> 0x80000000 (1G)
    .quad 0
    .quad 0
    .quad (0x80000 << 10) | 0xcf # VRWXAD
    .zero 8 * 508
    .quad (0x80000 << 10) | 0xcf # VRWXAD
