#include "regdef.h"

    .set noreorder
    .section .text.entry
    .globl _start
	.extern _root_page_table_buffer
	.extern _cur_kstack_ptr

_start:
    # setup stack and gp
    la    sp, bootstacktop
    la    gp, _gp

    la    t0, _cur_kstack_ptr
    la    t1, _root_page_table_buffer
    sw    t1, 0(t0)

    # set ebase
    la    t0, trap_entry
    mfc0  t1, $15, 1 # C0_EBASE
    or    t1, t1, t0
    mtc0  t1, $15, 1

    # exit bootstrap mode
    mfc0  t0, $12 # C0_STATUS
    li    t1, 0xFFBFFFFF # set BEV (bit 22) to 0
    and   t0, t0, t1 
    mtc0  t0, $12

    # directly jump to main function
    jal   rust_main
	nop

    .section .bss.stack
    .align 12  #PGSHIFT
    .global bootstack
bootstack:
    .space 4096 * 16 * 8
    .global bootstacktop
bootstacktop:

    .align 12  #PGSHIFT
    .global _root_page_table_buffer
_root_page_table_buffer:
    .space 1024 * 64 # 64KB
    .global _root_page_table_ptr
_root_page_table_ptr:
    .space 4 # 4bytes
    .global _cur_tls
_cur_tls:
    .space 4 # 4bytes
