#[cfg(feature = "use_generic_timer")]
mod generic_timer;
#[cfg(feature = "use_generic_timer")]
pub use self::generic_timer::GenericTimer as Timer;

#[cfg(not(feature = "use_generic_timer"))]
mod system_timer;
#[cfg(not(feature = "use_generic_timer"))]
pub use self::system_timer::SystemTimer as Timer;

/// The Raspberry Pi timer.
pub trait BasicTimer {
    /// Returns a new instance.
    fn new() -> Self;

    /// Initialization timer.
    fn init(&mut self);

    /// Reads the timer's counter and returns the 64-bit counter value.
    /// The returned value is the number of elapsed microseconds.
    fn read(&self) -> u64;

    /// Sets up a match in timer 1 to occur `us` microseconds from now. If
    /// interrupts for timer 1 are enabled and IRQs are unmasked, then a timer
    /// interrupt will be issued in `us` microseconds.
    fn tick_in(&mut self, us: u32);

    /// Returns `true` if timer interruption is pending. Otherwise, returns `false`.
    fn is_pending(&self) -> bool;
}

/// wait for `cycle` CPU cycles
#[inline(always)]
pub fn delay(cycle: u32) {
    for _ in 0..cycle {
        unsafe { asm!("nop") }
    }
}
