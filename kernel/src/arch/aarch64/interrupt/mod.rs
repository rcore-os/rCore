//! Interrupt and exception for aarch64.

mod context;
mod handler;
mod syndrome;

use aarch64::regs::*;

pub use self::context::*;
pub use self::handler::*;

/// Set the exception vector address
pub fn init() {
    extern "C" {
        fn __vectors();
    }
    VBAR_EL1.set(__vectors as u64);
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
    let daif = DAIF.get() as usize;
    disable();
    daif
}

/// Use the original status to restore the process
///
/// Arguments:
/// * flags:  original status(usize)
#[inline(always)]
pub unsafe fn restore(flags: usize) {
    DAIF.set(flags as u32);
}
