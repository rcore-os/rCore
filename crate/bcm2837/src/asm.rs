//! utility assembly instructions

/// delay for some clocks
#[inline]
pub unsafe fn delay(clock: u32) {
    #[cfg(target_arch = "aarch64")]
    asm!("1: subs x0, x0, #1; bne 1b;"
        :: "{x0}"(clock)
        :: "volatile");
}
