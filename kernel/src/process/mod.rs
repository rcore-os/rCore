pub use self::structs::*;
use crate::arch::cpu;
use crate::{
    consts::{MAX_CPU_NUM, MAX_PROCESS_NUM},
    memory::phys_to_virt,
    syscall::handle_syscall,
};
use alloc::{boxed::Box, sync::Arc};
use apic::LocalApic;
use log::*;
use trapframe::UserContext;

mod abi;
pub mod proc;
pub mod structs;
pub mod thread;

use crate::sync::SpinNoIrqLock as Mutex;
use apic::{XApic, LAPIC_ADDR};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
pub use proc::*;
pub use structs::*;
pub use thread::*;
use x86_64::{
    registers::control::{Cr2, Cr3, Cr3Flags},
    structures::paging::PhysFrame,
    PhysAddr,
};

pub fn init() {
    // create init process
    crate::shell::add_user_shell();

    info!("process: init end");
}

static mut THREADS: [Option<Arc<Thread>>; MAX_CPU_NUM] = [None; MAX_CPU_NUM];

/// Get current thread
///
/// `Thread` is a thread-local object.
/// It is safe to call this once, and pass `&mut Thread` as a function argument.
/// Should only be called in kernel trap handler
pub fn current_thread() -> Option<Arc<Thread>> {
    let cpu_id = cpu::id();
    unsafe { THREADS[cpu_id].clone() }
}

pub fn spawn(thread: Arc<Thread>) {
    let vmtoken = thread.vm.lock().token();
    let temp = thread.clone();
    let future = async move {
        loop {
            let mut cx = thread.begin_running();
            trace!("go to user: {:#x?}", cx);
            cx.run();
            trace!("back from user: {:#x?}", cx);

            let mut exit = false;
            match cx.trap_num {
                0x100 => exit = handle_syscall(&thread, &mut cx).await,
                0x20..=0x3f => {
                    let mut lapic = unsafe { XApic::new(phys_to_virt(LAPIC_ADDR)) };
                    lapic.eoi();
                    trace!("handle irq {}", cx.trap_num);
                    if cx.trap_num == 0x20 {
                        crate::trap::timer();
                    }
                    if cx.trap_num == 0x20 + 4 {
                        use crate::arch::driver::serial::*;
                        info!("\nInterupt: COM1");
                        crate::trap::serial(COM1.lock().receive());
                    }
                }
                0xe => {
                    // page fault
                    let addr = Cr2::read().as_u64();
                    debug!("page fault @ {:#x}", addr);

                    thread.vm.lock().handle_page_fault(addr as usize);
                }
                _ => {}
            }
            thread.end_running(cx);
            if exit {
                break;
            }
        }
    };

    spawn_thread(Box::pin(future), vmtoken, temp);
}

fn spawn_thread(
    future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    vmtoken: usize,
    thread: Arc<Thread>,
) {
    executor::spawn(PageTableSwitchWrapper {
        inner: Mutex::new(future),
        vmtoken,
        thread,
    });
}

struct PageTableSwitchWrapper {
    inner: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
    vmtoken: usize,
    thread: Arc<Thread>,
}

impl Future for PageTableSwitchWrapper {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // set cpu local thread
        // TODO: task local?
        let cpu_id = cpu::id();
        unsafe {
            THREADS[cpu_id] = Some(self.thread.clone());
        }
        // vmtoken won't change
        unsafe {
            Cr3::write(
                PhysFrame::containing_address(PhysAddr::new(self.vmtoken as u64)),
                Cr3Flags::empty(),
            );
        }
        let res = self.inner.lock().as_mut().poll(cx);
        unsafe {
            THREADS[cpu_id] = None;
        }
        res
    }
}
