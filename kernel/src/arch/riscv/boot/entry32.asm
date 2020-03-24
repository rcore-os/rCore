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

    # 2. paging
    # satp = (1 << 31) | PPN(boot_page_table_sv32)
    lui     t0, %hi(boot_page_table_sv32)
    li      t1, 0xc0000000 - 0x80000000
    sub     t0, t0, t1
    # 2.1 linear mapping (0xc0000000 -> 0x80000000)
    li      t2, 768*4
    li      t4, 0x400 << 10
    li      t5, 4
    add     t1, t0, t2
    li      t6, 1024*4
    add     t6, t0, t6
    li      t3, (0x80000 << 10) | 0xcf # VRWXAD
loop:
    sw      t3, 0(t1)
    add     t3, t3, t4
    add     t1, t1, t5
    bne     t1, t6, loop
    

    # 2.2 enable paging
    srli    t0, t0, 12
    li      t1, 1 << 31
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
boot_page_table_sv32:
    # NOTE: assume kernel image < 16M
    # 0x80000000 -> 0x80000000 (4M * 4)
    # 0xc0000000 -> 0x80000000 (mapped in code above)
    .zero 4 * 512
    .word (0x80000 << 10) | 0xcf # VRWXAD
    .word (0x80400 << 10) | 0xcf # VRWXAD
    .word (0x80800 << 10) | 0xcf # VRWXAD
    .word (0x80c00 << 10) | 0xcf # VRWXAD
    .zero 4 * 252
    .zero 4 * 256
