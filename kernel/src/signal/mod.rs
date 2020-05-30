use crate::process::{thread_manager, Process};
use crate::sync::SpinNoIrqLock as Mutex;
use alloc::sync::Arc;
use bitflags::*;
use num::FromPrimitive;

pub mod action;

#[derive(Eq, PartialEq, FromPrimitive, Debug, Copy, Clone)]
pub enum Signal {
    SIGHUP = 1,
    SIGINT = 2,
    SIGQUIT = 3,
    SIGILL = 4,
    SIGTRAP = 5,
    SIGABRT = 6,
    SIGBUS = 7,
    SIGFPE = 8,
    SIGKILL = 9,
    SIGUSR1 = 10,
    SIGSEGV = 11,
    SIGUSR2 = 12,
    SIGPIPE = 13,
    SIGALRM = 14,
    SIGTERM = 15,
    SIGSTKFLT = 16,
    SIGCHLD = 17,
    SIGCONT = 18,
    SIGSTOP = 19,
    SIGTSTP = 20,
    SIGTTIN = 21,
    SIGTTOU = 22,
    SIGURG = 23,
    SIGXCPU = 24,
    SIGXFSZ = 25,
    SIGVTALRM = 26,
    SIGPROF = 27,
    SIGWINCH = 28,
    SIGIO = 29,
    SIGPWR = 30,
    SIGSYS = 31,
    // real time signals
    SIGRT35 = 35,
    SIGRT36 = 36,
    SIGRT37 = 37,
    SIGRT38 = 38,
    SIGRT39 = 39,
    SIGRT40 = 40,
    SIGRT41 = 41,
    SIGRT42 = 42,
    SIGRT43 = 43,
    SIGRT44 = 44,
    SIGRT45 = 45,
    SIGRT46 = 46,
    SIGRT47 = 47,
    SIGRT48 = 48,
    SIGRT49 = 49,
    SIGRT50 = 50,
    SIGRT51 = 51,
    SIGRT52 = 52,
    SIGRT53 = 53,
    SIGRT54 = 54,
    SIGRT55 = 55,
    SIGRT56 = 56,
    SIGRT57 = 57,
    SIGRT58 = 58,
    SIGRT59 = 59,
    SIGRT60 = 60,
    SIGRT61 = 61,
    SIGRT62 = 62,
    SIGRT63 = 63,
    SIGRT64 = 64,
}

pub const SIGRTMIN: usize = 35;
pub const SIGRTMAX: usize = 64;

pub fn send_signal(process: Arc<Mutex<Process>>, sig: Signal, tid: isize) {
    let mut process = process.lock();
    process.signals[sig as usize] = Some(tid);
    if tid == -1 {
        info!("send {:?} to process {}", sig, process.pid);
        for &tid in process.threads.iter() {
            thread_manager().wakeup(tid);
        }
    } else {
        info!("send {:?} to thread {}", sig, tid);
        thread_manager().wakeup(tid as usize);
    }
}
