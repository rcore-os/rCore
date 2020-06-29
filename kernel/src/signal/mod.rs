use crate::arch::syscall::SYS_RT_SIGRETURN;
use crate::process::{process, process_of, Process, Thread};
use crate::sync::{Event, MutexGuard, SpinNoIrq, SpinNoIrqLock as Mutex};
use alloc::sync::Arc;
use bitflags::*;
use num::FromPrimitive;
use trapframe::{TrapFrame, UserContext};

mod action;

pub use self::action::*;

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
    process.eventbus.lock().set(Event::RECEIVE_SIGNAL);
    info!(
        "send signal {} to pid {} tid {}",
        info.signo, process.pid, tid
    )
}

/// See musl struct __ucontext
/// Not exactly the same for now
#[repr(C)]
#[derive(Clone)]
pub struct SignalUserContext {
    pub flags: usize,
    pub link: usize,
    pub stack: SignalStack,
    pub context: UserContext,
    pub sig_mask: Sigset,
}

#[repr(C)]
#[derive(Clone)]
pub struct SignalFrame {
    pub ret_code_addr: usize, // point to ret_code
    pub info: Siginfo,
    pub ucontext: SignalUserContext, // adapt interface, a little bit waste
    pub ret_code: [u8; 7],           // call sys_sigreturn
}

/// return whether this thread exits
pub fn handle_signal(thread: &Arc<Thread>, tf: &mut UserContext) -> bool {
    let mut process = thread.proc.lock();
    while let Some((idx, info)) =
        process
            .sig_queue
            .iter()
            .enumerate()
            .find_map(|(idx, &(info, tid))| {
                if (tid == -1 || tid as usize == thread.tid)
                    && !thread
                        .inner
                        .lock()
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
        info!("process {} received signal: {:?}", process.pid, signal);

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
                        return true;
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

                // save original signal alternate stack
                let stack = thread.inner.lock().signal_alternate_stack;

                let sig_sp = {
                    // use signal alternate stack when SA_ONSTACK is set
                    // fallback to default stack when unavailable
                    // man sigaction(2)
                    if action_flags.contains(SignalActionFlags::ONSTACK) {
                        let stack_flags = SignalStackFlags::from_bits_truncate(stack.flags);
                        if stack_flags.contains(SignalStackFlags::DISABLE) {
                            tf.get_sp()
                        } else {
                            let mut inner = thread.inner.lock();
                            inner.signal_alternate_stack.flags |= SignalStackFlags::ONSTACK.bits();

                            // handle auto disarm
                            if stack_flags.contains(SignalStackFlags::AUTODISARM) {
                                inner.signal_alternate_stack.flags |=
                                    SignalStackFlags::DISABLE.bits();
                            }

                            // top of stack
                            stack.sp + stack.size
                        }
                    } else {
                        tf.get_sp()
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
                frame.info = info;
                frame.ucontext = SignalUserContext {
                    flags: 0,
                    link: 0,
                    stack,
                    context: tf.clone(),
                    sig_mask: thread.inner.lock().sig_mask,
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
                    tf.general.rsp = sig_sp;
                    tf.general.rip = action.handler;

                    // pass handler argument
                    tf.general.rdi = info.signo as usize;
                    tf.general.rsi = &frame.info as *const Siginfo as usize;
                    tf.general.rdx = &frame.ucontext as *const SignalUserContext as usize;
                }
            }
        }
    }
    return false;
}

bitflags! {
    pub struct SignalStackFlags : u32 {
        const ONSTACK = 1;
        const DISABLE = 2;
        const AUTODISARM = 0x80000000;
    }
}

/// Linux struct stack_t
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SignalStack {
    pub sp: usize,
    pub flags: u32,
    pub size: usize,
}

impl Default for SignalStack {
    fn default() -> Self {
        // default to disabled
        SignalStack {
            sp: 0,
            flags: SignalStackFlags::DISABLE.bits,
            size: 0,
        }
    }
}
