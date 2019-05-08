// Interface for inter-processor interrupt.
// This module wraps inter-processor interrupt into a broadcast-calling style.

use crate::consts::KERNEL_OFFSET;
use crate::sync::{Semaphore, SpinLock as Mutex};
use alloc::boxed::Box;
use alloc::sync::Arc;
use apic::{LocalApic, XApic, LAPIC_ADDR};
use lazy_static::*;
use rcore_memory::Page;
use x86_64::instructions::tlb;
use x86_64::VirtAddr;

struct IPIInvoke<'a, A>(&'a (Fn(&A) -> ()), &'a A);

lazy_static! {
    static ref IPI_INVOKE_LOCK: Mutex<()> = Mutex::new(());
}

pub trait InvokeEventHandle {
    fn call(&self);
}

struct InvokeEvent<A: 'static> {
    function: fn(&A) -> (),
    argument: Arc<A>,
    done_semaphore: Arc<Semaphore>,
}

impl<A> InvokeEventHandle for InvokeEvent<A> {
    fn call(&self) {
        let arg_ref = self.argument.as_ref();
        (self.function)(arg_ref);
        self.done_semaphore.release();
    }
}

pub type IPIEventItem = Box<InvokeEventHandle>;

// TODO: something fishy is going on here...
// In fact, the argument lives as long as the Arc.
fn create_item<A: 'static>(f: fn(&A) -> (), arg: &Arc<A>, sem: &Arc<Semaphore>) -> IPIEventItem {
    Box::new(InvokeEvent {
        function: f,
        argument: arg.clone(),
        done_semaphore: sem.clone(),
    })
}
unsafe fn get_apic() -> XApic {
    let mut lapic = unsafe { XApic::new(KERNEL_OFFSET + LAPIC_ADDR) };
    lapic
}
pub fn invoke_on_allcpu<A: 'static>(f: fn(&A) -> (), arg: A, wait: bool) {
    // Step 1: initialize
    use super::interrupt::consts::IPIFuncCall;
    let mut apic = unsafe { get_apic() };
    let sem = Arc::new(Semaphore::new(0));
    let arcarg = Arc::new(arg);
    let mut cpu_count = 0;
    // Step 2: invoke
    super::gdt::Cpu::foreach(|cpu| {
        let id = cpu.get_id();
        cpu_count += 1;
        cpu.notify_event(create_item(f, &arcarg, &sem));
        apic.send_ipi(id as u8, IPIFuncCall);
    });
    if wait {
        for _ in 0..cpu_count {
            sem.acquire();
        }
    }
}

// Examples of such cases.

pub fn tlb_shootdown(tuple: &(usize, usize)) {
//    debug!("CPU {}: remote tlb flush {:x?}", super::cpu::id(), tuple);
    let (start_addr, end_addr) = *tuple;
    for p in Page::range_of(start_addr, end_addr) {
        tlb::flush(VirtAddr::new(p.start_address() as u64));
    }
}
