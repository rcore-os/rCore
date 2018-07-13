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
    let e = sstatus::read().sie() as usize;
    sstatus::clear_sie();
    e
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
    trace!("Interrupt: {:?}", tf.scause.cause());
    match tf.scause.cause() {
        Trap::Interrupt(SupervisorTimer) => timer(),
        _ => panic!("Unhandled interrupt: {:?}\n{:#010x?}", tf.scause.cause(), tf),
    }
    ::trap::before_return();
    trace!("Interrupt end");
}

fn timer() {
    ::trap::timer();
    super::timer::set_next();
}

extern {
    fn __alltraps();
}