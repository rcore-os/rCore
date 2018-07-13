section .text
global __alltraps
__alltraps:
    push rax
    push rcx
    push rdx
    push rdi
    push rsi
    push r8
    push r9
    push r10
    push r11

    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15

    mov rdi, rsp
    extern rust_trap
    call rust_trap

global trap_ret
trap_ret:

    mov rdi, rsp
    extern set_return_rsp
    call set_return_rsp

    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx

    pop r11
    pop r10
    pop r9
    pop r8
    pop rsi
    pop rdi
    pop rdx
    pop rcx
    pop rax

    ; pop trap_num, error_code
    add rsp, 16

    iretq