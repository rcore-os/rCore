.macro SAVE_ALL
    # If coming from userspace, preserve the user stack pointer and load
    # the kernel stack pointer. If we came from the kernel, sscratch
    # will contain 0, and we should continue on the current stack.
    csrrw sp, 0x140, sp     # sscratch
    bnez sp, _save_context
_restore_kernel_sp:
    csrr sp, 0x140          # sscratch
    # sscratch = previous-sp, sp = kernel-sp
_save_context:
    # provide room for trap frame
    addi sp, sp, -36 * 4
    # save x registers except x2 (sp)
    sw x1, 1*4(sp)
    sw x3, 3*4(sp)
    sw x4, 4*4(sp)
    sw x5, 5*4(sp)
    sw x6, 6*4(sp)
    sw x7, 7*4(sp)
    sw x8, 8*4(sp)
    sw x9, 9*4(sp)
    sw x10, 10*4(sp)
    sw x11, 11*4(sp)
    sw x12, 12*4(sp)
    sw x13, 13*4(sp)
    sw x14, 14*4(sp)
    sw x15, 15*4(sp)
    sw x16, 16*4(sp)
    sw x17, 17*4(sp)
    sw x18, 18*4(sp)
    sw x19, 19*4(sp)
    sw x20, 20*4(sp)
    sw x21, 21*4(sp)
    sw x22, 22*4(sp)
    sw x23, 23*4(sp)
    sw x24, 24*4(sp)
    sw x25, 25*4(sp)
    sw x26, 26*4(sp)
    sw x27, 27*4(sp)
    sw x28, 28*4(sp)
    sw x29, 29*4(sp)
    sw x30, 30*4(sp)
    sw x31, 31*4(sp)

    # get sp, sstatus, sepc, stval, scause
    # set sscratch = 0
    csrrw s0, 0x140, x0     # sscratch
    csrr s1, 0x100          # sstatus
    csrr s2, 0x141          # sepc
    csrr s3, 0x143          # stval
    csrr s4, 0x142          # scause
    # store sp, sstatus, sepc, sbadvaddr, scause
    sw s0, 2*4(sp)
    sw s1, 32*4(sp)
    sw s2, 33*4(sp)
    sw s3, 34*4(sp)
    sw s4, 35*4(sp)
.endm

.macro RESTORE_ALL
    lw s1, 32*4(sp)             # s1 = sstatus
    lw s2, 33*4(sp)             # s2 = sepc
    andi s0, s1, 1 << 8
    bnez s0, _restore_context   # back to U-mode? (sstatus.SPP = 1)
_save_kernel_sp:
    addi s0, sp, 36*4
    csrw 0x140, s0              # sscratch = kernel-sp
_restore_context:
    # restore sstatus, sepc
    csrw 0x100, s1
    csrw 0x141, s2

    # restore x registers except x2 (sp)
    lw x1, 1*4(sp)
    lw x3, 3*4(sp)
    lw x4, 4*4(sp)
    lw x5, 5*4(sp)
    lw x6, 6*4(sp)
    lw x7, 7*4(sp)
    lw x8, 8*4(sp)
    lw x9, 9*4(sp)
    lw x10, 10*4(sp)
    lw x11, 11*4(sp)
    lw x12, 12*4(sp)
    lw x13, 13*4(sp)
    lw x14, 14*4(sp)
    lw x15, 15*4(sp)
    lw x16, 16*4(sp)
    lw x17, 17*4(sp)
    lw x18, 18*4(sp)
    lw x19, 19*4(sp)
    lw x20, 20*4(sp)
    lw x21, 21*4(sp)
    lw x22, 22*4(sp)
    lw x23, 23*4(sp)
    lw x24, 24*4(sp)
    lw x25, 25*4(sp)
    lw x26, 26*4(sp)
    lw x27, 27*4(sp)
    lw x28, 28*4(sp)
    lw x29, 29*4(sp)
    lw x30, 30*4(sp)
    lw x31, 31*4(sp)
    # restore sp last
    lw x2, 2*4(sp)
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
    sret
