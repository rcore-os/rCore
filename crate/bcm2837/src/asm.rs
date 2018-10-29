//! utility assembly instructions

/// delay for some clocks
#[inline]
pub unsafe fn delay(_clock: u32) {
    #[cfg(target_arch = "aarch64")]
    asm!("mov x1, x0; 1: subs x1, x1, #1; bne 1b;"
        :: "{x0}"(_clock)
        :: "volatile");
}
