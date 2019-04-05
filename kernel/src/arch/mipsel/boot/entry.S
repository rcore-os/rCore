#include "regdef.h"

	.set noreorder
    .section .text.entry
    .globl _start

_start:
    # setup stack and gp
	la    sp, bootstacktop
    la    gp, _gp

    # set ebase
    la    t0, trap_entry
    mfc0  t1, $15 # C0_EBASE
    or    t1, t1, t0
    mtc0  t1, $15

    # exit bootstrap mode
    mfc0  t0, $12 # C0_STATUS
    li    t1, 0xFFBFFFFF # set BEV (bit 22) to 0
    and   t0, t0, t1 
    mtc0  t0, $12

    # directly jump to main function
    j rust_main
	nop

    .section .bss.stack
    .align 12  #PGSHIFT
    .global bootstack
bootstack:
    .space 4096 * 16 * 8
    .global bootstacktop
bootstacktop:
