#[cfg(feature = "use_generic_timer")]
mod generic_timer;
#[cfg(feature = "use_generic_timer")]
pub use self::generic_timer::Timer;

#[cfg(not(feature = "use_generic_timer"))]
mod system_timer;
#[cfg(not(feature = "use_generic_timer"))]
pub use self::system_timer::Timer;

/// Initialization timer
pub fn init() {
    Timer::new().init();
}

/// Returns the current time in microseconds.
pub fn current_time() -> u64 {
    Timer::new().read()
}

/// Sets up a match in timer 1 to occur `us` microseconds from now. If
/// interrupts for timer 1 are enabled and IRQs are unmasked, then a timer
/// interrupt will be issued in `us` microseconds.
pub fn tick_in(us: u32) {
    Timer::new().tick_in(us);
}

/// wait for `cycle` CPU cycles
#[inline(always)]
pub fn delay(cycle: u32) {
    for _ in 0..cycle {
        unsafe { asm!("nop") }
    }
}
