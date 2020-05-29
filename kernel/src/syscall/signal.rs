use crate::signal::action::{SigAction, Sigset};
use crate::signal::*;
use crate::syscall::SysError::EINVAL;
use crate::syscall::{SysResult, Syscall};

impl Syscall<'_> {
    pub fn sys_rt_sigaction(
        &self,
        signum: usize,
        act: *const SigAction,
        oldact: *mut SigAction,
        sigsetsize: usize,
    ) -> SysResult {
        info!(
            "rt_sigaction: signum: {:?}, act: {:?}, oldact: {:?}, sigsetsize: {}",
            signum, act, oldact, sigsetsize
        );
        if signum == SIGKILL || signum == SIGSTOP || sigsetsize != 8{
            return Err(EINVAL);
        }
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
}
