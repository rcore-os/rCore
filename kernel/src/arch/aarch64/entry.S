.section .text.entry
.globl _start

_start:
    ldr     x0, =bootstacktop
    mov     sp, x0

zero_bss:
    # load the start address and number of bytes in BSS section
    ldr     x1, =sbss
    ldr     x2, =__bss_length

zero_bss_loop:
    # zero out the BSS section, 64-bits at a time
    cbz     x2, zero_bss_loop_end
    str     xzr, [x1], #8
    sub     x2, x2, #8
    cbnz    x2, zero_bss_loop

zero_bss_loop_end:
    bl      rust_main
