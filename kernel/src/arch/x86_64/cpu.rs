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