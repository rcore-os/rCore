.section .text.boot
.globl _start

_start:
    # pc == 0x80000

    # read cpu affinity, start core 0, halt rest
    mrs     x1, mpidr_el1
    and     x1, x1, #3
    cbnz    x1, halt
    bl      el_setup
    bl      clear_bss
    bl      create_init_paging
    b       master_startup

halt:
    # core affinity != 0, halt it
    wfe
    b       halt

# switch to EL1, setup system registers of other EL
el_setup:
    # use SP_ELx for Exception level ELx
    msr     SPsel, #1

    # read the current exception level into x0 (ref: C5.2.1)
    mrs     x0, CurrentEL
    and     x0, x0, #0b1100
    lsr     x0, x0, #2
switch_to_el2:
    # switch to EL2 if we're in EL3. otherwise switch to EL1
    cmp     x0, #2
    beq     switch_to_el1

    # at EL3
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
    beq     el_setup_end

    # at EL2
    # set the temporary stack for EL1 in lower WA range
    adr     x0, _start
    msr     sp_el1, x0

    # set-up HCR_EL2, enable AArch64 in EL1 (bits 1, 31) (ref: D10.2.45)
    mov     x0, #0x0002
    movk    x0, #0x8000, lsl #16
    msr     hcr_el2, x0

    # don't trap accessing SVE registers (ref: D10.2.30)
    msr     cptr_el2, xzr

    # enable floating point and SVE (SIMD) (bits 20, 21) (ref: D10.2.29)
    mrs     x0, cpacr_el1
    orr     x0, x0, #(0x3 << 20)
    msr     cpacr_el1, x0

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
    msr     elr_el2, lr
    eret
el_setup_end:
    # at EL1
    adr     x0, _start
    mov     sp, x0
    ret

# enable paging, jump to upper VA range
master_startup:
    bl      enable_mmu
    ldr     x8, =jump_to_main
    br      x8

# set-up kernel stack, jump to main
jump_to_main:
    msr     ttbr0_el1, xzr
    ldr     x8, =bootstacktop
    mov     sp, x8
    mov     x29, xzr
    mov     x30, xzr
    b       rust_main

.section .bss.stack
.align 12
bootstack:
    .space 0x100000 // 1M
bootstacktop:

.section .data
.align 12
page_table_lvl4:
    .space 0x1000 // 4K
page_table_lvl3:
    .space 0x1000 // 4K
page_table_lvl2:
    .space 0x1000 // 4K
