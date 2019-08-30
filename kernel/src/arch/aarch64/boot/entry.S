.section .text.entry
.globl _start

_start:
    msr     ttbr0_el1, xzr
    ldr     x8, =bootstacktop
    mov     sp, x8
    bl      rust_main
1:  b       1b
