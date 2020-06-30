use crate::arch::cpu;
use crate::consts::INFORM_PER_MSEC;
use crate::process::*;
use crate::sync::SpinNoIrqLock as Mutex;
use crate::{signal::SignalUserContext, sync::Condvar};
use core::time::Duration;
use naive_timer::Timer;
use trapframe::TrapFrame;
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
    let now = crate::arch::timer::timer_now();
    NAIVE_TIMER.lock().expire(now);
}

pub fn serial(c: u8) {
    if c == b'\r' {
        // in linux, we use '\n' instead
        crate::fs::TTY.push(b'\n');
    } else {
        crate::fs::TTY.push(c);
    }
}
