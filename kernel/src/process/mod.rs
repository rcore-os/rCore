pub use self::structs::*;
use crate::arch::cpu;
use crate::consts::{MAX_CPU_NUM, MAX_PROCESS_NUM};
use alloc::{boxed::Box, sync::Arc};
use log::*;
use trapframe::UserContext;

mod abi;
pub mod structs;

pub fn init() {
    // create init process
    crate::shell::add_user_shell();

    info!("process: init end");
}

/// Get current thread
///
/// `Thread` is a thread-local object.
/// It is safe to call this once, and pass `&mut Thread` as a function argument.
pub unsafe fn current_thread() -> &'static mut Thread {
    // trick: force downcast from trait object
    //let (process, _): (&mut Thread, *const ()) = core::mem::transmute(processor().context());
    //process
    todo!()
}
