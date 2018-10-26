//! Interrupt handler implementation on raspi3.

pub use self::context::*;

#[path = "context.rs"]
mod context;

/// Initialize the trap to enable the interrupt.
pub fn init() {
    // TODO
    // info!("interrupt: init end");
}

/// Enable the interrupt.
#[inline(always)]
pub unsafe fn enable() {
    // TODO
}

/// Disable the interrupt.
#[inline(always)]
pub unsafe fn disable() {
    // TODO
}

/// Disable the interrupt and store the status.
///
/// return: status(usize)
#[inline(always)]
pub unsafe fn disable_and_store() -> usize {
    // TODO
    0
}

/// Use the original status to restore the process
///
/// Arguments:
/// * flags:  original status(usize)
#[inline(always)]
pub unsafe fn restore(flags: usize) {
    // TODO
}
