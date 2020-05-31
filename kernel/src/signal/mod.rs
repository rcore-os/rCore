use crate::process::{thread_manager, Process, current_thread, process_of, process};
use crate::sync::{SpinNoIrqLock as Mutex, MutexGuard, SpinNoIrq};
use alloc::sync::Arc;
use bitflags::*;
use num::FromPrimitive;

mod action;

pub use self::action::*;
use rcore_thread::std_thread::{current, yield_now};
use alloc::vec::Vec;
use crate::processor;
use crate::syscall::{SysError, SysResult};

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

// process and tid must be checked
pub fn send_signal(process: Arc<Mutex<Process>>,  tid: isize, signal: Signal) {
    process.lock().signals[signal as usize] = Some(tid);
    if tid == -1 {
        info!("send {:?} to process {}", signal, process.lock().pid.get());
        if let Some(current_tid) = processor().tid_option() {
            if process.lock().threads.contains(&current_tid) {
                drop(process);
                handle_signal();
            }
        } else {
            let process = process.lock();
            for &tid in process.threads.iter() {
                thread_manager().wakeup(tid);
            }
        }
    } else {
        info!("send {:?} to thread {}", signal, tid);
        if let Some(current_tid) = processor().tid_option() {
            drop(process);
            handle_signal();
        } else {
            thread_manager().wakeup(tid as usize);
        }
    }
}

#[inline(never)]
pub(crate) fn handle_signal() {
    let signals = unsafe { current_thread() }.proc.lock().signals.iter().enumerate().filter_map(|(signum, tid)| {
        // TODO: handle mask
        if let &Some(tid) = tid {
            if tid == -1 || tid as usize == current().id() {
                Some(signum)
            } else {
                None
            }
        } else {
            None
        }
    }).collect::<Vec<_>>();

    unsafe { current_thread() }.int = false;
    for signum in signals {
        use crate::signal::Flags;
        use Signal::*;

        let signal = <Signal as num::FromPrimitive>::from_usize(signum).unwrap();
        info!("received signal: {:?}", signal);
        let action = {
            let mut process = unsafe { current_thread() }.proc.lock();
            process.signals[signum] = None;
            process.dispositions[signum]
        };
        let flags = Flags::from_bits_truncate(action.flags);

        // enter signal handler
        match action.handler {
            x if x == SIG_DFL => {
                match signal {
                    SIGALRM | SIGHUP | SIGINT => {
                        info!("default action: Term");
                        // FIXME: exit code ref please?
                        unsafe { current_thread() }.proc.lock().exit(signum + 128);
                        yield_now();
                    }
                    _ => (),
                }
            }
            x if x == SIG_IGN => info!("ignore"),
            x if x == SIG_ERR => {
                // TODO
                unimplemented!();
            }
            _ => {
                unsafe { current_thread() }.int = true;
                if flags.contains(Flags::SA_SIGINFO) {
                    // TODO
                    unimplemented!();
                } else {
                    unsafe {
                        let handler: extern "C" fn(i32) = core::mem::transmute(action.handler);
                        handler(signum as i32);
                    }
                }
            }
        }
        if flags.contains(Flags::SA_RESTART) {
            // TODO: restart the syscall
            warn!("unsupported flag: {:?}", Flags::SA_RESTART);
        }
    }
}
