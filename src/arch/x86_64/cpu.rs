pub fn init() {
    enable_nxe_bit();
    enable_write_protect_bit();
}

/// Enable 'No-Execute' bit in page entry
pub fn enable_nxe_bit() {
    use x86_64::registers::model_specific::*;
    unsafe { Efer::update(|flags| flags.insert(EferFlags::NO_EXECUTE_ENABLE)); }
}

/// Enable write protection in kernel mode
pub fn enable_write_protect_bit() {
    use x86_64::registers::control::*;
    unsafe { Cr0::update(|flags| flags.insert(Cr0Flags::WRITE_PROTECT)); }
}

/// Exit qemu
/// See: https://wiki.osdev.org/Shutdown
/// Must run qemu with `-device isa-debug-exit`
/// The error code is `value written to 0x501` *2 +1, so it should be odd
pub unsafe fn exit_in_qemu(error_code: u8) -> ! {
    use x86_64::instructions::port::Port;
    assert_eq!(error_code & 1, 1, "error code should be odd");
    Port::new(0x501).write((error_code - 1) / 2);
    unreachable!()
}