use crate::process::{current_thread, process, process_of, thread_manager, Process};
use crate::sync::{MutexGuard, SpinNoIrq, SpinNoIrqLock as Mutex};
use alloc::sync::Arc;
use bitflags::*;
use num::FromPrimitive;

mod action;

pub use self::action::*;
use crate::arch::interrupt::TrapFrame;
use crate::arch::signal::MachineContext;
use crate::arch::syscall::SYS_RT_SIGRETURN;
use rcore_thread::std_thread::{current, yield_now};

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
    SIGRT32 = 32,
    SIGRT33 = 33,
    SIGRT34 = 34,
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

impl Signal {
    pub const RTMIN: usize = 32;
    pub const RTMAX: usize = 64;

    pub fn is_standard(self) -> bool {
        (self as usize) < Self::RTMIN
    }
}

// process and tid must be checked
pub fn send_signal(process: Arc<Mutex<Process>>, tid: isize, info: Siginfo) {
    let signal: Signal = <Signal as FromPrimitive>::from_i32(info.signo).unwrap();
    let mut process = process.lock();
    if signal.is_standard() && process.pending_sigset.contains(signal) {
        return;
    }
    process.sig_queue.push_back((info, tid));
    process.pending_sigset.add(signal);
    if tid == -1 {
        info!("send {:?} to process {}", signal, process.pid.get());
        for &tid in process.threads.iter() {
            // TODO: check mask here
            thread_manager().wakeup(tid);
        }
    } else {
        info!("send {:?} to thread {}", signal, tid);
        // TODO: check mask here
        thread_manager().wakeup(tid as usize);
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct SignalUserContext {
    pub flags: usize,
    pub link: usize,
    pub stack: SignalStack,
    pub mcontext: MachineContext,
    pub sig_mask: Sigset,
    pub _fpregs_mem: [usize; 64],
}

#[repr(C)]
#[derive(Clone)]
pub struct SignalFrame {
    pub ret_code_addr: usize, // point to ret_code
    pub tf: TrapFrame,
    pub info: Siginfo,
    pub ucontext: SignalUserContext, // adapt interface, a little bit waste
    pub ret_code: [u8; 7],     // call sys_sigreturn
}

pub fn has_signal_to_do() -> bool {
    let thread = unsafe { current_thread() };
    unsafe {
        current_thread()
            .proc
            .lock()
            .sig_queue
            .iter()
            .find(|(info, tid)| {
                let tid = *tid;
                (tid == -1 || tid as usize == current().id())
                    && !thread
                        .sig_mask
                        .contains(FromPrimitive::from_i32(info.signo).unwrap())
            })
            .is_some()
    }
}

pub fn do_signal(tf: &mut TrapFrame) {
    let thread = unsafe { current_thread() };
    let mut process = unsafe { current_thread().proc.lock() };
    while let Some((idx, info)) =
        process
            .sig_queue
            .iter()
            .enumerate()
            .find_map(|(idx, &(info, tid))| {
                if (tid == -1 || tid as usize == current().id())
                    && !thread
                        .sig_mask
                        .contains(FromPrimitive::from_i32(info.signo).unwrap())
                {
                    Some((idx, info))
                } else {
                    None
                }
            })
    {
        use crate::signal::SignalActionFlags;
        use Signal::*;

        let signal: Signal = <Signal as FromPrimitive>::from_i32(info.signo).unwrap();
        info!("received signal: {:?}", signal);

        process.sig_queue.remove(idx);
        process.pending_sigset.remove(signal);

        let action = process.dispositions[info.signo as usize];
        let action_flags = SignalActionFlags::from_bits_truncate(action.flags);

        // enter signal handler
        match action.handler {
            // TODO: complete default actions
            x if x == SIG_DFL => {
                match signal {
                    SIGALRM | SIGHUP | SIGINT => {
                        info!("default action: Term");
                        // FIXME: exit code ref please?
                        process.exit(info.signo as usize + 128);
                        yield_now();
                        unreachable!()
                    }
                    _ => (),
                }
            }
            x if x == SIG_IGN => {
                // TODO: handle SIGCHLD
                info!("ignore");
            }
            x if x == SIG_ERR => {
                // TODO
                unimplemented!();
            }
            _ => {
                info!("goto handler at {:#x}", action.handler);
                process.sigaltstack.flags |= SignalStackFlags::ONSTACK.bits();
                let stack = process.sigaltstack;
                let sig_sp = {
                    if action_flags.contains(SignalActionFlags::ONSTACK) {
                        let stack_flags = SignalStackFlags::from_bits_truncate(stack.flags);
                        if stack_flags.contains(SignalStackFlags::DISABLE) {
                            todo!()
                        } else {
                            stack.sp + stack.size
                        }
                    } else {
                        todo!()
                        //tf.get_sp()
                    }
                } - core::mem::size_of::<SignalFrame>();
                let frame = if let Ok(frame) = unsafe {
                    process
                        .vm
                        .lock()
                        .check_write_ptr(sig_sp as *mut SignalFrame)
                } {
                    frame
                } else {
                    unimplemented!()
                };
                frame.tf = tf.clone();
                frame.info = info;
                frame.ucontext = SignalUserContext {
                    flags: 0,
                    link: 0,
                    stack,
                    mcontext: MachineContext::from_tf(tf),
                    sig_mask: thread.sig_mask,
                    _fpregs_mem: [0; 64],
                };
                if action_flags.contains(SignalActionFlags::RESTORER) {
                    frame.ret_code_addr = action.restorer; // legacy
                } else {
                    frame.ret_code_addr = frame.ret_code.as_ptr() as usize;
                    // mov SYS_RT_SIGRETURN, %eax
                    frame.ret_code[0] = 0xb8;
                    // TODO: ref plz
                    unsafe {
                        *(frame.ret_code.as_ptr().add(1) as *mut u32) = SYS_RT_SIGRETURN as u32;
                    }
                    // syscall
                    frame.ret_code[5] = 0x0f;
                    frame.ret_code[6] = 0x05;
                }
                #[cfg(target_arch = "x86_64")]
                {
                    tf.rsp = sig_sp;
                    tf.rip = action.handler;

                    // pass handler argument
                    tf.rdi = info.signo as usize;
                    tf.rsi = &frame.info as *const Siginfo as usize;
                    // TODO: complete context
                    tf.rdx = &frame.ucontext as *const SignalUserContext as usize;
                }
                return;
            }
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
#[derive(Copy, Clone, Debug)]
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
