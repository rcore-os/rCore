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
use crate::arch::{get_sp, set_sp};
use crate::signal::SigInfo;
use alloc::alloc::handle_alloc_error;

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

// must be called with = user stack
// FIXME: set user mode?
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

    unsafe { current_thread().int = false; }
    for signum in signals {
        use crate::signal::SignalActionFlags;
        use Signal::*;

        let signal = <Signal as num::FromPrimitive>::from_usize(signum).unwrap();
        info!("received signal: {:?}", signal);
        let action = {
            let mut process = unsafe { current_thread().proc.lock() };
            process.signals[signum] = None;
            process.dispositions[signum]
        };
        let action_flags = SignalActionFlags::from_bits_truncate(action.flags);

        // enter signal handler
        match action.handler {
            // TODO: complete default actions
            x if x == SIG_DFL => {
                match signal {
                    SIGALRM | SIGHUP | SIGINT => {
                        info!("default action: Term");
                        // FIXME: exit code ref please?
                        unsafe { current_thread().proc.lock().exit(signum + 128); }
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
                unsafe { current_thread().int = true; }
                let sig_sp = {
                    if action_flags.contains(SignalActionFlags::ONSTACK) {
                        let stack = unsafe { current_thread().proc.lock().sigaltstack };
                        let stack_flags = SignalStackFlags::from_bits_truncate(stack.flags);
                        if !stack_flags.contains(SignalStackFlags::DISABLE) {
                            stack.sp + stack.size
                        } else {
                            unsafe {
                                current_thread().ustack_top
                            }
                        }
                    } else {
                        unsafe {
                            current_thread().ustack_top
                        }
                    }
                };
                if action_flags.contains(SignalActionFlags::SIGINFO) {
                    unsafe {
                        let action: extern "C" fn(i32, *mut SigInfo, usize) = core::mem::transmute(action.handler);
                        // TODO: complete info
                        let mut info = SigInfo {
                            signo: signum as i32,
                            errno: 0,
                            code: 0,
                            field: Default::default(),
                        };
                        // TODO: complete ucontext
                        // let mut sp = get_sp();
                        // sp = get_sp();
                        // set_sp(sig_sp);
                        action(signum as i32, &mut info as *mut SigInfo, 0);
                        // set_sp(sp);
                    }
                } else {
                    unsafe {
                        let handler: extern "C" fn(i32) = core::mem::transmute(action.handler);
                        let mut sp = get_sp();
                        sp = get_sp();  // get stack pointer after local variable `sp` is allocated
                        set_sp(sig_sp);
                        handler(signum as i32);
                        set_sp(sp);
                    }
                }
            }
        }
        if action_flags.contains(SignalActionFlags::RESTART) {
            // TODO: restart the syscall
            warn!("unsupported flag: {:?}", SignalActionFlags::RESTART);
        }
    }
}

bitflags! {
    pub struct SignalStackFlags : u32 {
        const ONSTACK = 1;
        const DISABLE = 2;
        const AUTODISARM = 0x80000000;
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SignalStack {
    pub sp: usize,
    pub flags: u32,
    pub size: usize,
}

impl Default for SignalStack {
    fn default() -> Self {
        SignalStack {
            sp: 0,
            flags: SignalStackFlags::DISABLE.bits,
            size: 0,
        }
    }
}
