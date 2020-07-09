pub mod consts;
mod handler;

pub use self::handler::*;
use crate::memory::phys_to_virt;
use crate::process::thread::Thread;
use alloc::sync::Arc;
use apic::*;
use trapframe::{TrapFrame, UserContext};

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
    llvm_asm!("pushfq; popq $0; cli" : "=r"(r) :: "memory");
    r
}

#[inline(always)]
pub unsafe fn restore(flags: usize) {
    llvm_asm!("pushq $0; popfq" :: "r"(flags) : "memory" "flags");
}

#[inline(always)]
pub fn no_interrupt(f: impl FnOnce()) {
    let flags = unsafe { disable_and_store() };
    f();
    unsafe { restore(flags) };
}

#[inline(always)]
pub fn enable_irq(irq: usize) {
    let mut ioapic = unsafe { IoApic::new(phys_to_virt(IOAPIC_ADDR as usize)) };
    ioapic.set_irq_vector(irq as u8, (consts::IrqMin + irq) as u8);
    ioapic.enable(irq as u8, 0);
}

pub fn timer() {
    crate::trap::timer();
}

#[inline(always)]
pub fn ack(_irq: usize) {
    let mut lapic = unsafe { XApic::new(phys_to_virt(LAPIC_ADDR)) };
    lapic.eoi();
}

pub fn get_trap_num(context: &UserContext) -> usize {
    context.trap_num
}

pub fn wait_for_interrupt() {
    x86_64::instructions::interrupts::enable_interrupts_and_hlt();
    x86_64::instructions::interrupts::disable();
}

pub fn handle_user_page_fault(thread: &Arc<Thread>, addr: usize) -> bool {
    thread.vm.lock().handle_page_fault(addr)
}

pub fn handle_reserved_inst(tf: &mut UserContext) -> bool {
    false
}
