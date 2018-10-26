# TODO rewrite in Rust, use crate cortex-a

.section .text.boot

.global _start
_start:
    # read cpu affinity, start core 0, halt rest
    mrs     x1, mpidr_el1
    and     x1, x1, #3
    cbz     x1, setup

halt:
    # core affinity != 0, halt it
    wfe
    b       halt

setup:
    # store the desired EL1 stack pointer in x1
    adr     x1, _start

    # read the current exception level into x0 (ref: C5.2.1)
    mrs     x0, CurrentEL
    and     x0, x0, #0b1100
    lsr     x0, x0, #2

switch_to_el2:
    # switch to EL2 if we're in EL3. otherwise switch to EL1
    cmp     x0, #2
    beq     switch_to_el1

    # set-up SCR_EL3 (bits 0, 4, 5, 7, 8, 10) (A53: 4.3.42)
    mov     x0, #0x5b1
    msr     scr_el3, x0

    # set-up SPSR_EL3 (bits 0, 3, 6, 7, 8, 9) (ref: C5.2.20)
    mov     x0, #0x3c9
    msr     spsr_el3, x0

    # switch
    adr     x0, switch_to_el1
    msr     elr_el3, x0

    eret

switch_to_el1:
    # switch to EL1 if we're not already in EL1. otherwise continue with start
    cmp     x0, #1
    beq     set_stack

    # set the stack-pointer for EL1
    msr     sp_el1, x1

    # set-up HCR_EL2, enable AArch64 in EL1 (bits 1, 31) (ref: D10.2.45)
    mov     x0, #0x0002
    movk    x0, #0x8000, lsl #16
    msr     hcr_el2, x0

    # Set SCTLR to known state (RES1: 11, 20, 22, 23, 28, 29) (ref: D10.2.100)
    mov     x0, #0x0800
    movk    x0, #0x30d0, lsl #16
    msr     sctlr_el1, x0

    # set-up SPSR_EL2 (bits 0, 2, 6, 7, 8, 9) (ref: C5.2.19)
    mov     x0, #0x3c5
    msr     spsr_el2, x0

    # enable CNTP for EL1/EL0 (ref: D7.5.2, D7.5.13)
    # NOTE: This doesn't actually enable the counter stream.
    mrs     x0, cnthctl_el2
    orr     x0, x0, #3
    msr     cnthctl_el2, x0
    msr     cntvoff_el2, xzr

    # switch
    adr     x0, set_stack
    msr     elr_el2, x0

    eret

set_stack:
    # set the current stack pointer
    mov     sp, x1

zero_bss:
    # load the start address and number of bytes in BSS section
    ldr     x1, =__bss_start
    ldr     x2, =__bss_length

zero_bss_loop:
    # zero out the BSS section, 64-bits at a time
    cbz     x2, go_kmain
    str     xzr, [x1], #8
    sub     x2, x2, #8
    cbnz    x2, zero_bss_loop

go_kmain:
    # jump to rust_main, which shouldn't return. halt if it does
    bl      rust_main
    b       halt
