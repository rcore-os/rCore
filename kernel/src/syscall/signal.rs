use crate::process::{PROCESSES, thread_manager, process_of};
use crate::process::{process, process_group};
use crate::signal::*;
use crate::syscall::SysError::{EINVAL, ESRCH};
use crate::syscall::{SysResult, Syscall};
use crate::thread;
use num::FromPrimitive;

impl Syscall<'_> {
    pub fn sys_rt_sigaction(
        &self,
        signum: usize,
        act: *const SigAction,
        oldact: *mut SigAction,
        sigsetsize: usize,
    ) -> SysResult {
        if let Some(signal) = <Signal as FromPrimitive>::from_usize(signum) {
            info!(
                "rt_sigaction: signum: {:?}, act: {:?}, oldact: {:?}, sigsetsize: {}",
                signal, act, oldact, sigsetsize
            );
            use Signal::*;
            if signal == SIGKILL || signal == SIGSTOP || sigsetsize != 8 {
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
            // let set = *unsafe { self.vm().check_read_ptr(set)? };
            let set = unsafe { self.vm().check_read_ptr(set)? };
            let set = *set; // prevent deadlock when page fault
            const BLOCK: usize = 0;
            const UNBLOCK: usize = 1;
            const SETMASK: usize = 2;
            match how {
                BLOCK => self.thread.sig_mask |= set,
                UNBLOCK => self.thread.sig_mask ^= self.thread.sig_mask & set,
                SETMASK => self.thread.sig_mask = set,
                _ => return Err(EINVAL),
            }
        }
        return Ok(0);
    }

    /// sending signal sig to process pid
    pub fn sys_kill(&mut self, pid: isize, signum: usize) -> SysResult {
        info!("kill: pid: {}, signum: {}", pid, signum);
        if let Some(sig) = num::FromPrimitive::from_usize(signum) {
            match pid {
                pid if pid > 0 => {
                    if let Some(process) = process(pid as usize) {
                        send_signal(process.lock(), sig, -1);
                    }
                }
                0 => {
                    let pgid = self.process().pgid;
                    for process in process_group(pgid).iter() {
                        send_signal(process.lock(), sig, -1);
                    }
                }
                -1 => {
                    // sig is sent to every process for which the calling process
                    // has permission to send signals, except for process 1 (init)
                    for process in PROCESSES.read().values() {
                        if let Some(process) = process.upgrade() {
                            send_signal(process.lock(), sig, -1);
                        }
                    }
                }
                _ => {
                    let pgid = -pid;
                    for process in process_group(pgid as i32).iter() {
                        send_signal(process.lock(), sig, -1);
                    }
                }
            }
            Ok(0)
        } else {
            Err(EINVAL)
        }
    }

    pub fn sys_tkill(&mut self, tid: usize, signum: usize) -> SysResult {
        let signal = FromPrimitive::from_usize(signum);
        if let Some(signal) = signal {
            info!("tkill: tid: {}, signal: {:?}", tid, signal);
            if let Some(process) = process_of(tid) {
                send_signal(process.lock(), signal, tid as isize);
                Ok(0)
            } else {
                Err(ESRCH)
            }
        } else {
            info!("tkill: tid: {}, signal: UNKNOWN", tid, );
            Err(EINVAL)
        }
    }
}
