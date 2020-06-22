//! Interrupt and exception for aarch64.

pub use self::handler::*;
use crate::arch::board::timer::is_pending;
use aarch64::regs::*;
use trapframe::UserContext;

pub mod consts;
pub mod handler;
mod syndrome;

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
    if is_pending() {
        crate::arch::board::timer::set_next();
        crate::trap::timer();
    }
}

pub fn ack(_irq: usize) {
    // TODO
}

pub fn get_trap_num(cx: &UserContext) -> usize {
    cx.trap_num
}

pub fn enable_irq(irq: usize) {
    // TODO
}
