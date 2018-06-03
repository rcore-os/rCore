; xv6 x86_64 entryother.S

; Each non-boot CPU ("AP") is started up in response to a STARTUP
; IPI from the boot CPU.  Section B.4.2 of the Multi-Processor
; Specification says that the AP will start in real mode with CS:IP
; set to XY00:0000, where XY is an 8-bit value sent with the
; STARTUP. Thus this code must start at a 4096-byte boundary.
;
; Because this code sets DS to zero, it must sit
; at an address in the low 2^16 bytes.
;
; Startothers (in main.c) sends the STARTUPs one at a time.
; It copies this code (start) at 0x7000.  It puts the address of
; a newly allocated per-core stack in start-4,the address of the
; place to jump to (mpenter) in start-8, and the physical address
; of entrypgdir in start-12.
;
; This code is identical to bootasm.S except:
;   - it does not need to enable A20
;   - it uses the address at start-4, start-8, and start-12

%define CR0_PE 1
%define STA_X       0x8     ; Executable segment
%define STA_E       0x4     ; Expand down (non-executable segments)
%define STA_C       0x4     ; Conforming code segment (executable only)
%define STA_W       0x2     ; Writeable (non-executable segments)
%define STA_R       0x2     ; Readable (executable segments)
%define STA_A       0x1     ; Accessed

extern other_main

section .text
bits 16
start:
    cli

    xor     ax, ax
    mov     ds, ax
    mov     es, ax
    mov     ss, ax

    lgdt    [gdt.desc]
    mov     eax, cr0
    or      eax, CR0_PE
    mov     cr0, eax

    jmp     gdt.code: start32

bits 32
start32:
    mov     ax, gdt.data
    mov     ds, ax
    mov     es, ax
    mov     ss, ax
    mov     ax, 0
    mov     fs, ax
    mov     gs, ax

    ; Switch to the stack allocated by startothers()
    mov     esp, [top - 4]

    call    enable_paging

    ; load the 64-bit GDT
    lgdt    [gdt64.pointer]

    jmp     gdt64.code: start64

error:
    mov     ax, 0x8a00
    mov     dx, ax
    out     dx, ax
    mov     ax, 0x8ae0
    out     dx, ax
spin:
    jmp     spin

enable_paging:
    ; load P4 to cr3 register (cpu uses this to access the P4 table)
    mov     eax, [top - 8]
    mov     cr3, eax

    ; enable PAE-flag in cr4 (Physical Address Extension)
    mov     eax, cr4
    or      eax, 1 << 5
    mov     cr4, eax

    ; set the long mode bit in the EFER MSR (model specific register)
    mov     ecx, 0xC0000080
    rdmsr
    or      eax, 1 << 8
    wrmsr

    ; enable paging in the cr0 register
    mov     eax, cr0
    or      eax, 1 << 31
    mov     cr0, eax

    ret

bits 64
start64:
    ; load 0 into all data segment registers
    mov     ax, 0
    mov     ss, ax
    mov     ds, ax
    mov     es, ax
    mov     fs, ax
    mov     gs, ax

    ; obtain kstack from data block before entryother
    mov     rsp, [top - 16]

    mov     rax, other_main
    call    rax

; section .rodata
align 4
gdt:
    ; NULL
    dw  0, 0
    db  0, 0, 0, 0
.code: equ $ - gdt
    dw  0xffff, 0
    db  0, (0x90 | STA_X | STA_R), 0xcf, 0
.data: equ $ - gdt
    dw  0xffff, 0
    db  0, (0x90 | STA_W), 0xcf, 0
.desc:
    dw  $ - gdt - 1
    dq  gdt

gdt64:
    dq  0 ; zero entry
.code: equ $ - gdt64 ; new
    dq  (1<<43) | (1<<44) | (1<<47) | (1<<53) ; code segment
.pointer:
    dw  $ - gdt64 - 1
    dq  gdt64

top: equ start + 0x1000