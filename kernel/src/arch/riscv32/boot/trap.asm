# Constants / Macros defined in Rust code:
#   XLENB
#   LOAD
#   STORE
#   TEST_BACK_TO_KERNEL

.macro SAVE_ALL
    # If coming from userspace, preserve the user stack pointer and load
    # the kernel stack pointer. If we came from the kernel, sscratch
    # will contain 0, and we should continue on the current stack.
    csrrw sp, sscratch, sp
    bnez sp, trap_from_user
trap_from_kernel:
    csrr sp, sscratch
    STORE gp, -36
    # sscratch = previous-sp, sp = kernel-sp
trap_from_user:
    # provide room for trap frame
    addi sp, sp, -36 * XLENB
    # save x registers except x2 (sp)
    STORE x1, 1
    STORE x3, 3
    STORE x4, 4
    STORE x5, 5
    STORE x6, 6
    STORE x7, 7
    STORE x8, 8
    STORE x9, 9
    STORE x10, 10
    STORE x11, 11
    STORE x12, 12
    STORE x13, 13
    STORE x14, 14
    STORE x15, 15
    STORE x16, 16
    STORE x17, 17
    STORE x18, 18
    STORE x19, 19
    STORE x20, 20
    STORE x21, 21
    STORE x22, 22
    STORE x23, 23
    STORE x24, 24
    STORE x25, 25
    STORE x26, 26
    STORE x27, 27
    STORE x28, 28
    STORE x29, 29
    STORE x30, 30
    STORE x31, 31

    # load hartid to gp from sp[0]
    LOAD gp, 0

    # get sp, sstatus, sepc, stval, scause
    # set sscratch = 0
    csrrw s0, sscratch, x0
    csrr s1, sstatus
    csrr s2, sepc
    csrr s3, stval
    csrr s4, scause
    # store sp, sstatus, sepc, sbadvaddr, scause
    STORE s0, 2
    STORE s1, 32
    STORE s2, 33
    STORE s3, 34
    STORE s4, 35
.endm

.macro RESTORE_ALL
    LOAD s1, 32             # s1 = sstatus
    LOAD s2, 33             # s2 = sepc
    andi s0, s1, 1 << 8     # sstatus.SPP = 1
    bnez s0, _to_kernel     # s0 = back to kernel?
_to_user:
    addi s0, sp, 36*XLENB
    csrw sscratch, s0       # sscratch = kernel-sp
    STORE gp, 0             # store hartid from gp to sp[0]
_to_kernel:
    # restore sstatus, sepc
    csrw sstatus, s1
    csrw sepc, s2

    # restore x registers except x2 (sp)
    LOAD x1, 1
    LOAD x3, 3
    LOAD x4, 4
    LOAD x5, 5
    LOAD x6, 6
    LOAD x7, 7
    LOAD x8, 8
    LOAD x9, 9
    LOAD x10, 10
    LOAD x11, 11
    LOAD x12, 12
    LOAD x13, 13
    LOAD x14, 14
    LOAD x15, 15
    LOAD x16, 16
    LOAD x17, 17
    LOAD x18, 18
    LOAD x19, 19
    LOAD x20, 20
    LOAD x21, 21
    LOAD x22, 22
    LOAD x23, 23
    LOAD x24, 24
    LOAD x25, 25
    LOAD x26, 26
    LOAD x27, 27
    LOAD x28, 28
    LOAD x29, 29
    LOAD x30, 30
    LOAD x31, 31
    # restore sp last
    LOAD x2, 2
.endm

    .section .text
    .globl trap_entry
trap_entry:
    SAVE_ALL
    mv a0, sp
    jal rust_trap
    .globl trap_return
trap_return:
    RESTORE_ALL
    # return from supervisor call
    sret
