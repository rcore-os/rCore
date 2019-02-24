.section .text
.global __alltraps
.intel_syntax noprefix

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

    # push fs.base
    xor rax, rax
    mov ecx, 0xC0000100
    rdmsr # msr[ecx] => edx:eax
    shl rdx, 32
    or rdx, rax
    push rdx

    mov rdi, rsp
    call rust_trap

.global trap_ret
trap_ret:

    mov rdi, rsp
    call set_return_rsp

    # pop fs.base
    pop rax
    mov rdx, rax
    shr rdx, 32
    mov ecx, 0xC0000100
    wrmsr # msr[ecx] <= edx:eax

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

    # pop trap_num, error_code
    add rsp, 16

    iretq