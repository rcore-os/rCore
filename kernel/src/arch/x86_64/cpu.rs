use super::apic::{LocalApic, XApic};
use super::raw_cpuid::CpuId;

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

pub fn id() -> usize {
    CpuId::new().get_feature_info().unwrap().initial_local_apic_id() as usize
}

pub fn send_ipi(cpu_id: usize) {
    let mut lapic = unsafe { XApic::new(0xffffff00_fee00000) };
    unsafe { lapic.send_ipi(cpu_id as u8, 0x30); } // TODO: Find a IPI trap num
}

pub fn init() {
    let mut lapic = unsafe { XApic::new(0xffffff00_fee00000) };
    lapic.cpu_init();
}

pub fn halt() {
    use x86_64::instructions::hlt;
    hlt();
}