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
pub unsafe fn disable_and_store() -> usize {
    let r: usize;
    asm!("pushfq; popq $0; cli" : "=r"(r) :: "memory");
    r
}

#[inline(always)]
pub unsafe fn restore(flags: usize) {
    asm!("pushq $0; popfq" :: "r"(flags) : "memory" "flags");
}

#[inline(always)]
pub fn no_interrupt(f: impl FnOnce()) {
    let flags = unsafe { disable_and_store() };
    f();
    unsafe { restore(flags) };
}

#[inline(always)]
pub fn enable_irq(irq: u8) {
    if cfg!(feature = "use_apic") {
        IOAPIC.lock().enable(irq, 0);
    } else {
        pic::enable_irq(irq);
    }
}