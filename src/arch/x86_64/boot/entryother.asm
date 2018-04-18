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

section .text
bits 16
start:
  cli

  xor    ax, ax
  mov    ds, ax
  mov    es, ax
  mov    ss, ax

  lgdt   [gdt.desc]
  mov    eax, cr0
  or     eax, CR0_PE
  mov    cr0, eax

;PAGEBREAK!
  jmp    gdt.kcode: start32

bits 32
start32:
  mov    ax, gdt.kdata
  mov    ds, ax
  mov    es, ax
  mov    ss, ax
  mov    ax, 0
  mov    fs, ax
  mov    gs, ax

  ; debug
  mov dword [0xb8000], 0x2f4b2f4f
  hlt

  ; defer paging until we switch to 64bit mode
  ; set ebx=1 so shared boot code knows we're booting a secondary core
  mov    ebx, 1

  ; Switch to the stack allocated by startothers()
  mov    esp, [start-4]
  ; Call mpenter()
  call	 [start-8]

  mov    ax, 0x8a00
  mov    dx, ax
  out    dx, ax
  mov    ax, 0x8ae0
  out    dx, ax
spin:
  jmp    spin

; section .rodata
align 4
gdt:
  ; NULL
  dw    0, 0
  db    0, 0, 0, 0
.kcode: equ $ - gdt
  dw    0xffff, 0
  db    0, (0x90 | STA_X | STA_R), 0xcf, 0
.kdata: equ $ - gdt
  dw    0xffff, 0
  db    0, (0x90 | STA_W), 0xcf, 0
.desc:
  dw   ($ - gdt - 1)
  dq   gdt