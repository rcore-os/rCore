use core::arch::x86_64::_rdtsc;

pub fn rand() -> u64 {
    // rdrand is not implemented in QEMU
    // so use rdtsc instead
    unsafe { _rdtsc() as u64 }
}
