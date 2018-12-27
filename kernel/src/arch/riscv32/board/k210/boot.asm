    .section .text.boot
boot:
	csrwi mie, 0
	csrwi mip, 0
    csrwi mscratch, 0
	csrwi medeleg, 0
	csrwi mideleg, 0

    // enable float unit
	li t0, 0x00006000 // MSTATUS_FS
    csrw mstatus, t0

    // uart init
    lui x1, 0x38000

    li t0, 3384
    sw t0, 0x18(x1)

    li t0, 1
    sw t0, 0x8(x1)
    sw t0, 0xc(x1)

    li t0, 3
    sw t0, 0x14(x1)
    sw x0, 0x10(x1)


1:  // test
    lw t0, 0(x1)
    blt t0, zero, 1b
    // write
    li t0, 0x21
    sw t0, 0(x1)

    csrr a0, mhartid
    // FIXME: enable core 1
    li a2, 0    // hart_mask
    j _start