use crate::arch::cpu;
use crate::arch::interrupt::{syscall, TrapFrame};
use crate::consts::INFORM_PER_MSEC;
use crate::process::*;
use crate::{signal::SignalUserContext, sync::Condvar};
use rcore_thread::std_thread as thread;
use rcore_thread::std_thread::current;
use naive_timer::Timer;
use core::time::Duration;
use spin::Mutex;
use trapframe::UserContext;

pub static mut TICK: usize = 0;

lazy_static! {
    pub static ref TICK_ACTIVITY: Condvar = Condvar::new();
}

pub fn uptime_msec() -> usize {
    unsafe { crate::trap::TICK * crate::consts::USEC_PER_TICK / 1000 }
}

lazy_static! {
    pub static ref NAIVE_TIMER: Mutex<Timer> = Mutex::new(Timer::default());
}

pub fn timer() {
    info!("timer");
    let now = crate::arch::timer::timer_now();
    NAIVE_TIMER.lock().expire(now);
}

pub fn error(tf: &UserContext) -> ! {
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
