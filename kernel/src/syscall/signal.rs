use crate::process::*;
use crate::signal::*;
use crate::syscall::SysError::{EINVAL, ENOMEM, EPERM, ESRCH};
use crate::syscall::{SysResult, Syscall};
use num::FromPrimitive;

impl Syscall<'_> {
    pub fn sys_rt_sigaction(
        &self,
        signum: usize,
        act: *const SignalAction,
        oldact: *mut SignalAction,
        sigsetsize: usize,
    ) -> SysResult {
        if let Some(signal) = <Signal as FromPrimitive>::from_usize(signum) {
            info!(
                "rt_sigaction: signum: {:?}, act: {:?}, oldact: {:?}, sigsetsize: {}",
                signal, act, oldact, sigsetsize
            );
            use Signal::*;
            if signal == SIGKILL
                || signal == SIGSTOP
                || sigsetsize != core::mem::size_of::<Sigset>()
            {
                Err(EINVAL)
            } else {
                let mut proc = self.process();
                if !oldact.is_null() {
                    let oldact = unsafe { self.vm().check_write_ptr(oldact)? };
                    *oldact = proc.dispositions[signum];
                }
                if !act.is_null() {
                    let act = unsafe { self.vm().check_read_ptr(act)? };
                    info!("new action: {:?}", act);
                    proc.dispositions[signum] = *act;
                }
                Ok(0)
            }
        } else {
            info!(
                "rt_sigaction: sigal: UNKNOWN, act: {:?}, oldact: {:?}, sigsetsize: {}",
                act, oldact, sigsetsize
            );
            Err(EINVAL)
        }
    }

    pub fn sys_rt_sigreturn(&mut self) -> SysResult {
        info!("rt_sigreturn");
        // FIXME: adapt arch
        //let frame = unsafe { &*((self.tf.get_sp() - 8) as *mut SignalFrame) };
        let frame: SignalFrame = todo!();
        // frame.info.signo
        /*
        {
            let mut process = self.process();
            process.sigaltstack.flags ^=
                process.sigaltstack.flags & SignalStackFlags::ONSTACK.bits();
        }
        */

        // *self.tf = TrapFrame::from_mcontext(&frame.ucontext.mcontext);
        todo!();
        //*self.tf = frame.tf.clone();
        /*
        let mc = &frame.ucontext.mcontext;
        self.tf.general.r15 = mc.r15;
        self.tf.general.r14 = mc.r14;
        self.tf.general.r13 = mc.r13;
        self.tf.general.r12 = mc.r12;
        self.tf.general.rbp = mc.rbp;
        self.tf.general.rbx = mc.rbx;
        self.tf.general.r11 = mc.r11;
        self.tf.general.r10 = mc.r10;
        self.tf.general.r9 = mc.r9;
        self.tf.general.r8 = mc.r8;
        self.tf.general.rsi = mc.rsi;
        self.tf.general.rdi = mc.rdi;
        self.tf.general.rdx = mc.rdx;
        self.tf.general.rcx = mc.rcx;
        self.tf.general.rax = mc.rax;
        self.tf.general.rip = mc.rip;
        self.tf.general.rsp = mc.rsp;
        */

        let ret = self.context.general.rax as isize;
        if ret >= 0 {
            Ok(ret as usize)
        } else {
            Err(FromPrimitive::from_isize(-ret).unwrap())
        }
    }

    pub fn sys_rt_sigprocmask(
        &mut self,
        how: usize,
        set: *const Sigset,
        oldset: *mut Sigset,
        sigsetsize: usize,
    ) -> SysResult {
        info!(
            "rt_sigprocmask: how: {}, set: {:?}, oldset: {:?}, sigsetsize: {}",
            how, set, oldset, sigsetsize
        );
        if sigsetsize != 8 {
            return Err(EINVAL);
        }
        if !oldset.is_null() {
            let oldset = unsafe { self.vm().check_write_ptr(oldset)? };
            *oldset = self.thread.sig_mask;
        }
        if !set.is_null() {
            let set = unsafe { self.vm().check_read_ptr(set)? };
            const BLOCK: usize = 0;
            const UNBLOCK: usize = 1;
            const SETMASK: usize = 2;
            /*
            match how {
                BLOCK => self.thread.sig_mask.add_set(set),
                UNBLOCK => self.thread.sig_mask.remove_set(set),
                SETMASK => self.thread.sig_mask = *set,
                _ => return Err(EINVAL),
            }
            */
        }
        return Ok(0);
    }

    /// sending signal sig to process pid
    pub fn sys_kill(&mut self, pid: isize, signum: usize) -> SysResult {
        if let Some(signal) = <Signal as FromPrimitive>::from_usize(signum) {
            info!("kill: pid: {}, signal: {:?}", pid, signal);
            let info = Siginfo {
                signo: signum as i32,
                errno: 0,
                code: SI_USER,
                field: Default::default(),
            };
            match pid {
                pid if pid > 0 => {
                    if let Some(process) = process(pid as usize) {
                        send_signal(process, -1, info);
                        Ok(0)
                    } else {
                        Err(ESRCH)
                    }
                }
                0 => {
                    let pgid = self.process().pgid;
                    for process in process_group(pgid) {
                        send_signal(process, -1, info);
                    }
                    Ok(0)
                }
                -1 => {
                    // TODO: check permissions
                    // sig is sent to every process for which the calling process
                    // has permission to send signals, except for process 1 (init)
                    for process in PROCESSES.read().values() {
                        send_signal(process.clone(), -1, info);
                    }
                    Ok(0)
                }
                _ => {
                    let process_group = process_group((-pid) as Pgid);
                    if process_group.is_empty() {
                        Err(ESRCH)
                    } else {
                        for process in process_group {
                            send_signal(process, -1, info);
                        }
                        Ok(0)
                    }
                }
            }
        } else {
            info!("kill: pid: {}, signal: UNKNOWN", pid);
            Err(EINVAL)
        }
    }

    pub fn sys_tkill(&mut self, tid: usize, signum: usize) -> SysResult {
        if let Some(signal) = <Signal as FromPrimitive>::from_usize(signum) {
            info!("tkill: tid: {}, signal: {:?}", tid, signal);
            if let Some(process) = process_of(tid) {
                send_signal(
                    process,
                    tid as isize,
                    Siginfo {
                        signo: signum as i32,
                        errno: 0,
                        code: SI_TKILL,
                        field: Default::default(),
                    },
                );
                Ok(0)
            } else {
                Err(ESRCH)
            }
        } else {
            info!("tkill: tid: {}, signum: {}", tid, signum);
            Err(EINVAL)
        }
    }

    pub fn sys_sigaltstack(&self, ss: *const SignalStack, old_ss: *mut SignalStack) -> SysResult {
        info!("sigaltstack: ss: {:?}, old_ss: {:?}", ss, old_ss);
        const MINSIGSTKSZ: usize = 2048;
        if !old_ss.is_null() {
            let old_ss = unsafe { self.vm().check_write_ptr(old_ss)? };
            *old_ss = self.process().sigaltstack;
        }
        if !ss.is_null() {
            let ss = unsafe { self.vm().check_read_ptr(ss)? };
            info!("new stack: {:?}", ss);

            if ss.flags & 2 != 0 && ss.size < MINSIGSTKSZ {
                return Err(ENOMEM);
            }
            // only allow SS_AUTODISARM or SS_DISABLE
            if ss.flags ^ (ss.flags & 0x8000002) != 0 {
                return Err(EINVAL);
            }

            let old_ss = &mut self.process().sigaltstack;
            let flags = SignalStackFlags::from_bits_truncate(old_ss.flags);
            if flags.contains(SignalStackFlags::ONSTACK) {
                return Err(EPERM);
            }
            *old_ss = *ss;
        }
        Ok(0)
    }
}
