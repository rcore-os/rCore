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

pub fn error(tf: &TrapFrame) -> ! {
    error!("{:#x?}", tf);
    let pid = processor().pid();
    error!("On CPU{} Process {}", cpu::id(), pid);

    processor().manager().exit(pid, 0x100);
    processor().yield_now();
    unreachable!();
}