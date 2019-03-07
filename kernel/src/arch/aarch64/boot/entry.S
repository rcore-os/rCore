.section .text.entry
.globl _start

_start:
    ldr     x0, =bootstacktop
    mov     sp, x0
    bl      rust_main
