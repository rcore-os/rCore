use super::riscv::register::*;
pub use self::context::*;

#[path = "context.rs"]
mod context;

pub fn init() {
    unsafe {
        // Set sscratch register to 0, indicating to exception vector that we are
        // presently executing in the kernel
        sscratch::write(0);
        // Set the exception vector address
        stvec::write(__alltraps as usize, stvec::TrapMode::Direct);
    }
    info!("interrupt: init end");
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
        Trap::Exception(UserEnvCall) => syscall(tf),
        _ => ::trap::error(tf),
    }
    ::trap::before_return();
    trace!("Interrupt end");
}

fn timer() {
    ::trap::timer();
    super::timer::set_next();
}

fn syscall(tf: &mut TrapFrame) {
    tf.sepc += 4;   // Must before syscall, because of fork.
    let ret = ::syscall::syscall(tf.x[10], [tf.x[11], tf.x[12], tf.x[13], tf.x[14], tf.x[15], tf.x[16]], tf);
    tf.x[10] = ret as usize;
}

extern {
    fn __alltraps();
}