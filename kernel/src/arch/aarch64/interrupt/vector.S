.section .text

.macro HANDLER source kind
    .align 7
    stp     lr, x0, [sp, #-16]!
    mov     x0, #\source
    movk    x0, #\kind, lsl #16
    b       __alltraps
.endm

.global __vectors
.align 11
__vectors:
    HANDLER 0 0
    HANDLER 0 1
    HANDLER 0 2
    HANDLER 0 3
    HANDLER 1 0
    HANDLER 1 1
    HANDLER 1 2
    HANDLER 1 3
    HANDLER 2 0
    HANDLER 2 1
    HANDLER 2 2
    HANDLER 2 3
    HANDLER 3 0
    HANDLER 3 1
    HANDLER 3 2
    HANDLER 3 3
