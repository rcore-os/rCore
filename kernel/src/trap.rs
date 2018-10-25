use process::*;
use arch::interrupt::TrapFrame;
use arch::cpu;

pub static mut TICK: usize = 0;

pub fn timer() {
    processor().tick();
    if cpu::id() == 0 {
        unsafe { TICK += 1; }
    }
}

pub fn before_return() {
}

pub fn error(tf: &TrapFrame) -> ! {
    let pid = processor().pid();
    error!("On CPU{} Process {}:\n{:#x?}", cpu::id(), pid, tf);

    processor().manager().exit(pid, 0x100);
    processor().yield_now();
    unreachable!();
}