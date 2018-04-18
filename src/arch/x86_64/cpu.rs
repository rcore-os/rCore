pub fn init() {
    enable_nxe_bit();
    enable_write_protect_bit();
}

/// Enable 'No-Execute' bit in page entry
pub fn enable_nxe_bit() {
    use x86_64::registers::msr::{IA32_EFER, rdmsr, wrmsr};

    let nxe_bit = 1 << 11;
    // The EFER register is only allowed in kernel mode
    // But we are in kernel mode. So it's safe.
    unsafe {
        let efer = rdmsr(IA32_EFER);
        wrmsr(IA32_EFER, efer | nxe_bit);
    }
}

/// Enable write protection in kernel mode
pub fn enable_write_protect_bit() {
    use x86_64::registers::control_regs::{cr0, cr0_write, Cr0};

    // The CR0 register is only allowed in kernel mode
    // But we are in kernel mode. So it's safe.
    unsafe { cr0_write(cr0() | Cr0::WRITE_PROTECT) };
}

/// Exit qemu
/// See: https://wiki.osdev.org/Shutdown
/// Must run qemu with `-device isa-debug-exit`
/// The error code is `value written to 0x501` *2 +1, so it should be odd
pub unsafe fn exit_in_qemu(error_code: u8) -> ! {
    use x86_64::instructions::port::outb;
    assert_eq!(error_code & 1, 1, "error code should be odd");
    outb(0x501, (error_code - 1) / 2);
    unreachable!()
}