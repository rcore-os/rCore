use crate::arch::cpu;
use crate::consts::INFORM_PER_MSEC;
use crate::process::*;
use crate::sync::SpinNoIrqLock as Mutex;
use crate::{signal::SignalUserContext, sync::Condvar};
use core::sync::atomic::{AtomicUsize, Ordering};
use core::time::Duration;
use naive_timer::Timer;
use trapframe::TrapFrame;
use trapframe::UserContext;
pub static TICK: AtomicUsize = AtomicUsize::new(0);
pub static TICK_ALL_PROCESSORS: AtomicUsize = AtomicUsize::new(0);

pub unsafe fn wall_tick() -> usize {
    return TICK.load(Ordering::Relaxed);
}
pub fn cpu_tick() -> usize {
    return TICK_ALL_PROCESSORS.load(Ordering::Relaxed);
}
pub fn do_tick() {
    if crate::arch::cpu::id() == 0 {
        let ret = TICK.fetch_add(1, Ordering::Relaxed);
    }
    TICK_ALL_PROCESSORS.fetch_add(1, Ordering::Relaxed);
}
lazy_static! {
    pub static ref TICK_ACTIVITY: Condvar = Condvar::new();
}

pub fn uptime_msec() -> usize {
    unsafe { crate::trap::wall_tick() * crate::consts::USEC_PER_TICK / 1000 }
}

lazy_static! {
    pub static ref NAIVE_TIMER: Mutex<Timer> = Mutex::new(Timer::default());
}

pub fn timer() {
    do_tick();
    //let ret=unsafe{wall_tick()};

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
