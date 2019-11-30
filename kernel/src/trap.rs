use crate::arch::cpu;
use crate::arch::interrupt::TrapFrame;
use crate::consts::INFORM_PER_MSEC;
use crate::process::*;
use crate::sync::Condvar;
use rcore_thread::std_thread as thread;

pub static mut TICK: usize = 0;

lazy_static! {
    pub static ref TICK_ACTIVITY: Condvar = Condvar::new();
}

pub fn uptime_msec() -> usize {
    unsafe { crate::trap::TICK * crate::consts::USEC_PER_TICK / 1000 }
}

pub fn timer() {
    if cpu::id() == 0 {
        unsafe {
            TICK += 1;
            if uptime_msec() % INFORM_PER_MSEC == 0 {
                TICK_ACTIVITY.notify_all();
            }
        }
    }
    processor().tick();
}

pub fn error(tf: &TrapFrame) -> ! {
    error!("{:#x?}", tf);
    unsafe {
        let mut proc = current_thread().proc.lock();
        proc.exit(0x100);
    }
    thread::yield_now();
    unreachable!();
}

pub fn serial(c: char) {
    if c == '\r' {
        // in linux, we use '\n' instead
        crate::fs::STDIN.push('\n');
    } else {
        crate::fs::STDIN.push(c);
    }
}
