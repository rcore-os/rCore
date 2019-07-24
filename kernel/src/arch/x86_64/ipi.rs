//! Interface for inter-processor interrupt.
//! This module wraps inter-processor interrupt into a broadcast-calling style.

use crate::memory::phys_to_virt;
use alloc::boxed::Box;
use alloc::sync::Arc;
use apic::{LocalApic, XApic, LAPIC_ADDR};
use core::sync::atomic::{spin_loop_hint, AtomicU8, Ordering};

pub type IPIEventItem = Box<dyn Fn()>;

unsafe fn get_apic() -> XApic {
    let mut lapic = XApic::new(phys_to_virt(LAPIC_ADDR));
    lapic
}

pub fn invoke_on_allcpu(f: impl Fn() + 'static, wait: bool) {
    // Step 1: initialize
    use super::interrupt::consts::IPIFuncCall;
    let mut apic = unsafe { get_apic() };
    let func = Arc::new(f);
    let cpu_count = super::gdt::Cpu::iter().count();
    let rest_count = Arc::new(AtomicU8::new(cpu_count as u8));
    // Step 2: invoke
    for cpu in super::gdt::Cpu::iter() {
        let func_clone = func.clone();
        let rest_clone = rest_count.clone();
        cpu.notify_event(Box::new(move || {
            func_clone();
            rest_clone.fetch_sub(1, Ordering::Relaxed);
        }));
        apic.send_ipi(cpu.id() as u8, IPIFuncCall);
    }
    if wait {
        // spin if remote invocation do not complete
        while rest_count.load(Ordering::Relaxed) != 0 {
            spin_loop_hint();
        }
    }
}
