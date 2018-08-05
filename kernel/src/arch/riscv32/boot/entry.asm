    .section .entry
    .global _start
_start:
	csrwi 0x304, 0      # mie
	csrwi 0x344, 0      # mip
    csrwi 0x340, 0      # mscratch
	csrwi 0x180, 0      # satp
	li t0, -1
	csrw 0x302, t0      # medeleg
	csrw 0x303, t0      # mideleg
	csrw 0x306, t0      # mcounteren
	csrw 0x106, t0      # scounteren
    li t0, 1 << 11      # MPP = S
    csrw 0x300, t0      # mstatus
    lui  t0, %hi(rust_main)
    addi t0, t0, %lo(rust_main)
    csrw 0x341, t0      # mepc
    lui  sp, %hi(bootstacktop)
    addi sp, sp, %lo(bootstacktop)
	mret

    .section .bss
    .align 12  #PGSHIFT
    .global bootstack
bootstack:
    .space 4096 * 16  #KSTACKSIZE
    .global bootstacktop
bootstacktop:
