use x86_64;
use arch::driver::{apic::IOAPIC, pic};

pub mod consts;
mod handler;
mod trapframe;

pub use self::trapframe::*;
pub use self::handler::*;

#[inline(always)]
pub unsafe fn enable() {
    x86_64::instructions::interrupts::enable();
}

#[inline(always)]
pub unsafe fn disable() {
    x86_64::instructions::interrupts::disable();
}

#[inline(always)]
pub fn enable_irq(irq: u8) {
    if cfg!(feature = "use_apic") {
        IOAPIC.lock().enable(irq, 0);
    } else {
        pic::enable_irq(irq);
    }
}