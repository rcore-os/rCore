.section .text.boot
.globl _start

_start:
    # read cpu affinity, start core 0, halt the rest
    mfc0    $8, $15, 1
    beqz    $8, setup
    andi    $8, $8, 0x3ff # use bits 11 ~ 0

halt:
    # core affinity != 0, halt it
    b       halt
    nop

setup:
    # put the bootstack at 8MB offset of physical mem
    la     $29, 0x80800000 # $sp
    la     $28, _gp        # $gp
    b      boot_main
    nop