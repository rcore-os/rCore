use process::*;
use arch::interrupt::TrapFrame;
use arch::cpu;

pub fn timer() {
    processor().tick();
}

pub fn before_return() {
}

pub fn error(tf: &TrapFrame) -> ! {
    let pid = processor().pid();
    error!("On CPU{} Process {}:\n{:#x?}", cpu::id(), pid, tf);

    processor().manager().set_status(pid, Status::Exited(0x100));
    processor().yield_now();
    unreachable!();
}