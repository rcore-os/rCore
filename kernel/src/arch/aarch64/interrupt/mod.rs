//! Interrupt and exception for aarch64.

pub mod consts;
mod handler;
mod syndrome;

use aarch64::regs::*;
use trapframe::UserContext;

pub use self::handler::*;

/// Set the exception vector address
pub fn init() {
    extern "C" {
        fn __vectors();
    }
    //VBAR_EL1.set(__vectors as u64);
}

/// Enable the interrupt (only IRQ).
#[inline(always)]
pub unsafe fn enable() {
    llvm_asm!("msr daifclr, #2");
}

/// Disable the interrupt (only IRQ).
#[inline(always)]
pub unsafe fn disable() {
    llvm_asm!("msr daifset, #2");
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

pub fn timer() {
    // TODO
}

pub fn ack(_irq: usize) {
    // TODO
}

pub fn get_trap_num(cx: &UserContext) -> usize {
    // TODO
    0
}

pub fn enable_irq(irq: usize) {
    // TODO
}
