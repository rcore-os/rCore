#include "regdef.h"

	.set noreorder
    .section .text.entry
    .globl _start

_start:
	la sp, bootstacktop
    la gp, _gp

    b rust_main
	nop

    .section .bss.stack
    .align 12  #PGSHIFT
    .global bootstack
bootstack:
    .space 4096 * 16 * 8
    .global bootstacktop
bootstacktop:
