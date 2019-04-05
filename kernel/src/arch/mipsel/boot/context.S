#include "regdef.h"

.set noat
.set noreorder

// .section .text.context
.globl switch_context
.extern root_page_table_ptr

switch_context:
	// save from's registers
	addi  sp, sp, (-4*14)
	sw sp, 0(a0)
	sw ra, 0(sp)
	sw s0, 2*4(sp)
	sw s1, 3*4(sp)
	sw s2, 4*4(sp)
	sw s3, 5*4(sp)
	sw s4, 6*4(sp)
	sw s5, 7*4(sp)
	sw s6, 8*4(sp)
	sw s7, 9*4(sp)
	sw s8, 10*4(sp)
	sw gp, 11*4(sp)
	sw ra, 12*4(sp)
	sw sp, 13*4(sp)

	la s0, root_page_table_ptr
	lw at, 0(s0)
	sw at, 4(sp)

	// restore to's registers
	lw sp, 0(a1)
	lw at, 4(sp)
	sw at, 0(s0)

	lw ra, 0(sp)
	lw s0, 2*4(sp)
	lw s1, 3*4(sp)
	lw s2, 4*4(sp)
	lw s3, 5*4(sp)
	lw s4, 6*4(sp)
	lw s5, 7*4(sp)
	lw s6, 8*4(sp)
	lw s7, 9*4(sp)
	lw s8, 10*4(sp)
	lw gp, 11*4(sp)
	lw ra, 12*4(sp)
	lw sp, 13*4(sp)
	addi sp, sp, (4*14)

	sw zero, 0(a1)
	jr ra
	nop
