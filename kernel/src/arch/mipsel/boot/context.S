#include "regdef.h"

.set noat
.set noreorder

.section .text.context
.globl switch_context
.extern _root_page_table_ptr
.extern _cur_kstack_ptr
.extern _cur_tls

switch_context:
	// save from's registers
	addi  sp, sp, (-4*16)
	sw sp, 0(a0)
	sw ra, 0(sp)
	sw s0, 4*4(sp)
	sw s1, 5*4(sp)
	sw s2, 6*4(sp)
	sw s3, 7*4(sp)
	sw s4, 8*4(sp)
	sw s5, 9*4(sp)
	sw s6, 10*4(sp)
	sw s7, 11*4(sp)
	sw s8, 12*4(sp)
	sw gp, 13*4(sp)
	// sw ra, 12*4(sp)
	// sw sp, 13*4(sp)

	// save page table address
	la s0, _root_page_table_ptr
	lw s1, 0(s0)
	sw s1, 4(sp)

	// save TLS
	la s2, _cur_tls
	lw s1, 0(s2)
	sw s1, 2*4(sp)

	// restore to's registers
	lw sp, 0(a1)

	// restore page table address
	lw s1, 4(sp)
	sw s1, 0(s0)

	// restore TLS
	lw s1, 2*4(sp)
	sw s1, 0(s2)
	mtc0 s1, $4, 2  // cp0.user_local 

	lw ra, 0(sp)
	lw s0, 4*4(sp)
	lw s1, 5*4(sp)
	lw s2, 6*4(sp)
	lw s3, 7*4(sp)
	lw s4, 8*4(sp)
	lw s5, 9*4(sp)
	lw s6, 10*4(sp)
	lw s7, 11*4(sp)
	lw s8, 12*4(sp)
	lw gp, 13*4(sp)
	addi sp, sp, (4*16)

	sw zero, 0(a1)
	jr ra
	nop
