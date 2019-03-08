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

    # save fp registers
    # align to 16 byte boundary
    sub rsp, 512
    mov rax, rsp
    and rax, 0xFFFFFFFFFFFFFFF0
    # fxsave (rax)
    .byte 0x0f
    .byte 0xae
    .byte 0x00
    mov rbx, rsp
    sub rbx, rax
    # push fp state offset
    sub rsp, 16
    push rbx

    mov rdi, rsp
    call rust_trap

.global trap_ret
trap_ret:

    mov rdi, rsp
    call set_return_rsp

    # pop fp state offset
    pop rbx
    cmp rbx, 16 # only 0-15 are valid
    jge skip_fxrstor
    mov rax, rsp
    add rax, 16
    sub rax, rbx
    # fxrstor (rax)
    .byte 0x0f
    .byte 0xae
    .byte 0x08
skip_fxrstor:
    add rsp, 16+512

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