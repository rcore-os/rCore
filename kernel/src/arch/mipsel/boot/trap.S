#include "regdef.h"

	.set noat
	.set noreorder
    .section .text.ebase
    .globl trap_entry

.org 0x0
trap_entry:
# +0x000: TLB-miss vector
	b general_trap_vec

# +0x180: general vector
.org 0x180
general_trap_vec:
	move k1, sp         # save stack pointer to k1
	mfc0 k0, $12        # read cp0.status
	andi k0, k0, 0x10   # extract cp0.status.ksu
	beq  k0, zero, trap_from_kernel
	nop  # delayslot

trap_from_user:
	# load kstack, we can use k0 to store something
#	la     k0, kernel_stack
#	la     sp, kernel_stack_top
	la     k0, _cur_kstack_ptr
	lw     sp, 0(k0)

trap_from_kernel:
	/* 
	 *  k0 is damaged
	 *  k1 = old stack pointer
	 *  sp = kernel stack */
	
	# allocate 38 words for trapframe + 4 extra words
	addiu sp, sp, -168

	# save general registers
	sw ra, 160(sp)
	sw fp, 156(sp)
	sw k1, 152(sp)  # k1 = old sp
	sw gp, 148(sp)
	sw k1, 144(sp)  # real k1 is damaged
	sw k0, 140(sp)	# real k0 is damaged
	sw t9, 136(sp)
	sw t8, 132(sp)
	sw s7, 128(sp)
	sw s6, 124(sp)
	sw s5, 120(sp)
	sw s4, 116(sp)
	sw s3, 112(sp)
	sw s2, 108(sp)
	sw s1, 104(sp)
	sw s0, 100(sp)
	sw t7, 96(sp)
	sw t6, 92(sp)
	sw t5, 88(sp)
	sw t4, 84(sp)
	sw t3, 80(sp)
	sw t2, 76(sp)
	sw t1, 72(sp)
	sw t0, 68(sp)
	sw a3, 64(sp)
	sw a2, 60(sp)
	sw a1, 56(sp)
	sw a0, 52(sp)
	sw v1, 48(sp)
	sw v0, 44(sp)
	sw AT, 40(sp)
	nop	

	# save hi/lo
	mflo t1
	sw t1, 36(sp)
	mfhi t0
	sw t0, 32(sp)

	# save special registers
	mfc0 t0, $8     # cp0.vaddr
	sw t0, 28(sp)

	mfc0 t1, $14    # cp0.epc
	sw t1, 24(sp)

	mfc0 t0, $13    # cp0.cause
	sw t0, 20(sp)

	mfc0 t1, $12    # cp0.status
	sw t1, 16(sp)

	# support nested interrupt
	la   t0, ~0x1b   # reset status.ksu, status.exl, status.ie
	and  t1, t1, t0
	mtc0 t1, $12     # cp0.status

	# prepare to call rust_trap
	ori a0, sp, 0             /* set argument (trapframe) */
	jal rust_trap
	nop

	.globl trap_return
trap_return:
	# restore special registers
	lw   t1, 16(sp)
	ori  t1, t1, 0x2  # status.exl
	nop
	mtc0 t1, $12      # cp0.status

	lw k0, 24(sp)
	mtc0 k0, $14      # cp0.epc

	lw t0, 32(sp)
	mthi t0
	lw t1, 36(sp)
	mtlo t1

	# restore general registers
	lw AT, 40(sp)
	lw v0, 44(sp)
	lw v1, 48(sp)
	lw a0, 52(sp)
	lw a1, 56(sp)
	lw a2, 60(sp)
	lw a3, 64(sp)
	lw t0, 68(sp)
	lw t1, 72(sp)
	lw t2, 76(sp)
	lw t3, 80(sp)
	lw t4, 84(sp)
	lw t5, 88(sp)
	lw t6, 92(sp)
	lw t7, 96(sp)
	lw s0, 100(sp)
	lw s1, 104(sp)
	lw s2, 108(sp)
	lw s3, 112(sp)
	lw s4, 116(sp)
	lw s5, 120(sp)
	lw s6, 124(sp)
	lw s7, 128(sp)
	lw t8, 132(sp)
	lw t9, 136(sp)

	# lw k0, 140(sp)
	# lw k1, 144(sp)
	lw gp, 148(sp)
	lw fp, 156(sp)
	lw ra, 160(sp)

	lw sp, 152(sp)

	eret
	nop

    .section .bss.stack
    .align 12  #PGSHIFT
    .global kernel_stack
kernel_stack:
    .space 1024 * 16 # 16KB for kernel stack
    .global kernel_stack_top
kernel_stack_top:

    .align 12  #PGSHIFT
    .global _root_page_table_buffer
_root_page_table_buffer:
    .space 1024 * 64 # 64KB
    .global _root_page_table_ptr
_root_page_table_ptr:
    .space 4 # 4bytes
    .global _cur_kstack_ptr
_cur_kstack_ptr:
    .space 4 # 4bytes
