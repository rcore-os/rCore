use super::riscv::register::*;
pub use self::context::*;

#[path = "context.rs"]
mod context;

pub fn init() {
    unsafe {
        // Set the exception vector address
        stvec::write(__alltraps as usize, stvec::TrapMode::Direct);
    }
    info!("stvec: init end");
}

#[inline(always)]
pub unsafe fn enable() {
    sstatus::set_sie();
}

#[inline(always)]
pub unsafe fn disable_and_store() -> usize {
    sstatus::read().sie() as usize
}

#[inline(always)]
pub unsafe fn restore(flags: usize) {
    if flags != 0 {
        sstatus::set_sie();
    }
}

#[no_mangle]
pub extern fn rust_trap(tf: &mut TrapFrame) {
    use super::riscv::register::scause::{Trap, Interrupt, Exception};
    match tf.scause.cause() {
        Trap::Interrupt(SupervisorTimer) => timer(),
        _ => panic!("Unhandled interrupt: {:?}\n{:#x?}", tf.scause.cause(), tf),
    }
}

fn timer() {
    ::timer_interrupt();
    super::timer::set_next();
}

extern {
    fn __alltraps();
}