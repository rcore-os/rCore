.section .text

.macro SAVE_ALL
    # lr, x0 is saved in HANDLER
    str     x29, [sp, #-16]!
    stp     x27, x28, [sp, #-16]!
    stp     x25, x26, [sp, #-16]!
    stp     x23, x24, [sp, #-16]!
    stp     x21, x22, [sp, #-16]!
    stp     x19, x20, [sp, #-16]!
    stp     x17, x18, [sp, #-16]!
    stp     x15, x16, [sp, #-16]!
    stp     x13, x14, [sp, #-16]!
    stp     x11, x12, [sp, #-16]!
    stp     x9, x10, [sp, #-16]!
    stp     x7, x8, [sp, #-16]!
    stp     x5, x6, [sp, #-16]!
    stp     x3, x4, [sp, #-16]!
    stp     x1, x2, [sp, #-16]!

    # stp     q30, q31, [sp, #-32]!
    # stp     q28, q29, [sp, #-32]!
    # stp     q26, q27, [sp, #-32]!
    # stp     q24, q25, [sp, #-32]!
    # stp     q22, q23, [sp, #-32]!
    # stp     q20, q21, [sp, #-32]!
    # stp     q18, q19, [sp, #-32]!
    # stp     q16, q17, [sp, #-32]!
    # stp     q14, q15, [sp, #-32]!
    # stp     q12, q13, [sp, #-32]!
    # stp     q10, q11, [sp, #-32]!
    # stp     q8, q9, [sp, #-32]!
    # stp     q6, q7, [sp, #-32]!
    # stp     q4, q5, [sp, #-32]!
    # stp     q2, q3, [sp, #-32]!
    # stp     q0, q1, [sp, #-32]!

    mrs     x2, tpidr_el0
    mrs     x1, sp_el0
    stp     x1, x2, [sp, #-16]!

    mrs     x2, spsr_el1
    mrs     x1, elr_el1
    stp     x1, x2, [sp, #-16]!
.endm

.macro RESTORE_ALL
    ldp     x1, x2, [sp], #16
    msr     elr_el1, x1
    msr     spsr_el1, x2

    ldp     x1, x2, [sp], #16
    msr     sp_el0, x1
    msr     tpidr_el0, x2

    # ldp     q0, q1, [sp], #32
    # ldp     q2, q3, [sp], #32
    # ldp     q4, q5, [sp], #32
    # ldp     q6, q7, [sp], #32
    # ldp     q8, q9, [sp], #32
    # ldp     q10, q11, [sp], #32
    # ldp     q12, q13, [sp], #32
    # ldp     q14, q15, [sp], #32
    # ldp     q16, q17, [sp], #32
    # ldp     q18, q19, [sp], #32
    # ldp     q20, q21, [sp], #32
    # ldp     q22, q23, [sp], #32
    # ldp     q24, q25, [sp], #32
    # ldp     q26, q27, [sp], #32
    # ldp     q28, q29, [sp], #32
    # ldp     q30, q31, [sp], #32

    ldp     x1, x2, [sp], #16
    ldp     x3, x4, [sp], #16
    ldp     x5, x6, [sp], #16
    ldp     x7, x8, [sp], #16
    ldp     x9, x10, [sp], #16
    ldp     x11, x12, [sp], #16
    ldp     x13, x14, [sp], #16
    ldp     x15, x16, [sp], #16
    ldp     x17, x18, [sp], #16
    ldp     x19, x20, [sp], #16
    ldp     x21, x22, [sp], #16
    ldp     x23, x24, [sp], #16
    ldp     x25, x26, [sp], #16
    ldp     x27, x28, [sp], #16
    ldr     x29, [sp], #16
    ldp     lr, x0, [sp], #16
.endm

.global __alltraps
__alltraps:
    SAVE_ALL

    # x0 is set in HANDLER
    mrs x1, esr_el1
    mov x2, sp
    bl rust_trap

.global __trapret
__trapret:
    RESTORE_ALL
    eret
