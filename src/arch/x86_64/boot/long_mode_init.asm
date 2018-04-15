global long_mode_start
extern rust_main

KERNEL_OFFSET equ 0xffff_ff00_0000_0000

section .text
bits 64
long_mode_start:
    ; load 0 into all data segment registers
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; translate rsp to virtual address
    mov rax, KERNEL_OFFSET
    add rsp, rax

    ; call the rust main
    extern rust_main
    mov rax, rust_main
    call rax

    ; print `OKAY` to screen
    mov rax, 0x2f592f412f4b2f4f
    mov qword [0xb8000], rax
    hlt
