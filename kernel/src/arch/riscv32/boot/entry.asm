    .section .entry
    .global _start
_start:
	csrw mie, 0
	csrw mip, 0
    csrw mscratch, 0
	csrw satp, 0
	li t0, -1
	csrw medeleg, t0
	csrw mideleg, t0
	csrw mcounteren, t0
	csrw scounteren, t0
    li t0, 1 << 11      ; MPP = S
    csrw mstatus, t0
    la t0, rust_main
    csrw mepc, t0
    la sp, bootstacktop
	mret

    .section .bss
    .align 12  #PGSHIFT
    .global bootstack
bootstack:
    .space 4096 * 16  #KSTACKSIZE
    .global bootstacktop
bootstacktop:
