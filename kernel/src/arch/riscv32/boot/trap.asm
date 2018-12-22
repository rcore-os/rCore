# Constants / Macros defined in Rust code:
#   xscratch
#   xstatus
#   xepc
#   xcause
#   xtval
#   XRET

.macro SAVE_ALL
    # If coming from userspace, preserve the user stack pointer and load
    # the kernel stack pointer. If we came from the kernel, sscratch
    # will contain 0, and we should continue on the current stack.
    csrrw sp, (xscratch), sp
    bnez sp, _save_context
_restore_kernel_sp:
    csrr sp, (xscratch)
    # sscratch = previous-sp, sp = kernel-sp
_save_context:
    # provide room for trap frame
    addi sp, sp, -36 * XLENB
    # save x registers except x2 (sp)
    sw x1, 1*XLENB(sp)
    sw x3, 3*XLENB(sp)
    # tp(x4) = hartid. DON'T change.
    # sw x4, 4*XLENB(sp)
    sw x5, 5*XLENB(sp)
    sw x6, 6*XLENB(sp)
    sw x7, 7*XLENB(sp)
    sw x8, 8*XLENB(sp)
    sw x9, 9*XLENB(sp)
    sw x10, 10*XLENB(sp)
    sw x11, 11*XLENB(sp)
    sw x12, 12*XLENB(sp)
    sw x13, 13*XLENB(sp)
    sw x14, 14*XLENB(sp)
    sw x15, 15*XLENB(sp)
    sw x16, 16*XLENB(sp)
    sw x17, 17*XLENB(sp)
    sw x18, 18*XLENB(sp)
    sw x19, 19*XLENB(sp)
    sw x20, 20*XLENB(sp)
    sw x21, 21*XLENB(sp)
    sw x22, 22*XLENB(sp)
    sw x23, 23*XLENB(sp)
    sw x24, 24*XLENB(sp)
    sw x25, 25*XLENB(sp)
    sw x26, 26*XLENB(sp)
    sw x27, 27*XLENB(sp)
    sw x28, 28*XLENB(sp)
    sw x29, 29*XLENB(sp)
    sw x30, 30*XLENB(sp)
    sw x31, 31*XLENB(sp)

    # get sp, sstatus, sepc, stval, scause
    # set sscratch = 0
    csrrw s0, (xscratch), x0
    csrr s1, (xstatus)
    csrr s2, (xepc)
    csrr s3, (xtval)
    csrr s4, (xcause)
    # store sp, sstatus, sepc, sbadvaddr, scause
    sw s0, 2*XLENB(sp)
    sw s1, 32*XLENB(sp)
    sw s2, 33*XLENB(sp)
    sw s3, 34*XLENB(sp)
    sw s4, 35*XLENB(sp)
.endm

.macro RESTORE_ALL
    lw s1, 32*XLENB(sp)             # s1 = sstatus
    lw s2, 33*XLENB(sp)             # s2 = sepc
    andi s0, s1, 1 << 8
    bnez s0, _restore_context   # back to S-mode? (sstatus.SPP = 1)
_save_kernel_sp:
    addi s0, sp, 36*XLENB
    csrw (xscratch), s0         # sscratch = kernel-sp
_restore_context:
    # restore sstatus, sepc
    csrw (xstatus), s1
    csrw (xepc), s2

    # restore x registers except x2 (sp)
    lw x1, 1*XLENB(sp)
    lw x3, 3*XLENB(sp)
    # lw x4, 4*XLENB(sp)
    lw x5, 5*XLENB(sp)
    lw x6, 6*XLENB(sp)
    lw x7, 7*XLENB(sp)
    lw x8, 8*XLENB(sp)
    lw x9, 9*XLENB(sp)
    lw x10, 10*XLENB(sp)
    lw x11, 11*XLENB(sp)
    lw x12, 12*XLENB(sp)
    lw x13, 13*XLENB(sp)
    lw x14, 14*XLENB(sp)
    lw x15, 15*XLENB(sp)
    lw x16, 16*XLENB(sp)
    lw x17, 17*XLENB(sp)
    lw x18, 18*XLENB(sp)
    lw x19, 19*XLENB(sp)
    lw x20, 20*XLENB(sp)
    lw x21, 21*XLENB(sp)
    lw x22, 22*XLENB(sp)
    lw x23, 23*XLENB(sp)
    lw x24, 24*XLENB(sp)
    lw x25, 25*XLENB(sp)
    lw x26, 26*XLENB(sp)
    lw x27, 27*XLENB(sp)
    lw x28, 28*XLENB(sp)
    lw x29, 29*XLENB(sp)
    lw x30, 30*XLENB(sp)
    lw x31, 31*XLENB(sp)
    # restore sp last
    lw x2, 2*XLENB(sp)
.endm

    .section .text
    .globl __alltraps
__alltraps:
    SAVE_ALL
    mv a0, sp
    jal rust_trap
    .globl __trapret
__trapret:
    RESTORE_ALL
    # return from supervisor call
    XRET
