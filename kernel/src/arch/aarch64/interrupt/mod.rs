//! Interrupt and exception for aarch64.

mod handler;
mod context;
mod syndrome;

pub use self::context::*;
pub use self::handler::*;

/// Set the exception vector address
pub fn init() {
    unsafe {
        asm!(
            "adr x0, __vectors;
             msr vbar_el1, x0"
        );
    }
}

/// Enable the interrupt (only IRQ).
#[inline(always)]
pub unsafe fn enable() {
    asm!("msr daifclr, #2");
}

/// Disable the interrupt (only IRQ).
#[inline(always)]
pub unsafe fn disable() {
    asm!("msr daifset, #2");
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
