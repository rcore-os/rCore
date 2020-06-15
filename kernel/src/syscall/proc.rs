//! Syscalls for process

use super::*;
use crate::fs::FileLike;
use crate::signal::{has_signal_to_do, send_signal, Signal};
use crate::syscall::SysError::{EINTR, ESRCH};
use alloc::sync::Weak;

impl Syscall<'_> {
    /// Fork the current process. Return the child's PID.
    pub fn sys_fork(&mut self) -> SysResult {
        let new_thread = self.thread.fork(self.tf);
        let pid = new_thread.proc.lock().pid.get();
        let tid = thread_manager().add(new_thread);
        thread_manager().detach(tid);
        info!("fork: {} -> {}", self.process().pid, pid);
        Ok(pid)
    }

    #[cfg(target_arch = "x86_64")]
    pub fn sys_vfork(&mut self) -> SysResult {
        self.sys_fork()
    }

    /// Create a new thread in the current process.
    /// The new thread's stack pointer will be set to `newsp`,
    /// and thread pointer will be set to `newtls`.
    /// The child tid will be stored at both `parent_tid` and `child_tid`.
    /// This is partially implemented for musl only.
    pub fn sys_clone(
        &mut self,
        flags: usize,
        newsp: usize,
        parent_tid: *mut u32,
        child_tid: *mut u32,
        newtls: usize,
    ) -> SysResult {
        let clone_flags = CloneFlags::from_bits_truncate(flags);
        info!(
            "clone: flags: {:?} == {:#x}, newsp: {:#x}, parent_tid: {:?}, child_tid: {:?}, newtls: {:#x}",
            clone_flags, flags, newsp, parent_tid, child_tid, newtls
        );
        if flags == 0x4111 || flags == 0x11 {
            warn!("sys_clone is calling sys_fork instead, ignoring other args");
            return self.sys_fork();
        }
        if (flags != 0x7d0f00) && (flags != 0x5d0f00) {
            //0x5d0f00 is the args from gcc of alpine linux
            //warn!("sys_clone only support musl pthread_create");
            panic!(
                "sys_clone only support sys_fork OR musl pthread_create without flags{:x}",
                flags
            );
            //return Err(SysError::ENOSYS);
        }
        let parent_tid_ref = unsafe { self.vm().check_write_ptr(parent_tid)? };
        // child_tid buffer should not be set because CLONE_CHILD_SETTID flag is not specified in the current implementation
        // let child_tid_ref = unsafe { self.vm().check_write_ptr(child_tid)? };
        let mut new_thread = self
            .thread
            .clone(self.tf, newsp, newtls, child_tid as usize);
        if clone_flags.contains(CloneFlags::CHILD_CLEARTID) {
            new_thread.clear_child_tid = child_tid as usize;
        }
        let tid = thread_manager().add(new_thread);
        thread_manager().detach(tid);
        info!("clone: {} -> {}", thread::current().id(), tid);
        *parent_tid_ref = tid as u32;
        // *child_tid_ref = tid as u32;
        Ok(tid)
    }

    /// Wait for the process exit.
    /// Return the PID. Store exit code to `wstatus` if it's not null.
    pub fn sys_wait4(&mut self, pid: isize, wstatus: *mut i32) -> SysResult {
        info!("wait4: pid: {}, code: {:?}", pid, wstatus);
        let wstatus = if !wstatus.is_null() {
            Some(unsafe { self.vm().check_write_ptr(wstatus)? })
        } else {
            None
        };
        #[derive(Debug)]
        enum WaitFor {
            AnyChild,
            AnyChildInGroup,
            Pid(usize),
        }
        let target = match pid {
            -1 => WaitFor::AnyChild,
            0 => WaitFor::AnyChildInGroup,
            p if p > 0 => WaitFor::Pid(p as usize),
            _ => unimplemented!(),
        };
        loop {
            let mut proc = self.process();
            // check child_exit_code
            let find = match target {
                WaitFor::AnyChild | WaitFor::AnyChildInGroup => proc
                    .child_exit_code
                    .iter()
                    .next()
                    .map(|(&pid, &code)| (pid, code)),
                WaitFor::Pid(pid) => proc.child_exit_code.get(&pid).map(|&code| (pid, code)),
            };
            // if found, return
            if let Some((pid, exit_code)) = find {
                proc.child_exit_code.remove(&pid);
                {
                    let mut process_table = PROCESSES.write();
                    process_table.remove(&pid);
                }
                if let Some(wstatus) = wstatus {
                    *wstatus = exit_code as i32;
                }
                return Ok(pid);
            }
            // if not, check pid
            let invalid = {
                let children = proc
                    .children
                    .iter()
                    .filter_map(|(pid, weak)| {
                        if weak.upgrade().is_none() {
                            None
                        } else {
                            Some(pid)
                        }
                    })
                    .collect::<Vec<_>>();
                match target {
                    WaitFor::AnyChild | WaitFor::AnyChildInGroup => children.len() == 0,
                    WaitFor::Pid(pid) => children.iter().find(|p| p.get() == pid).is_none(),
                }
            };
            if invalid {
                return Err(SysError::ECHILD);
            }
            info!(
                "wait: thread {} -> {:?}, sleep",
                thread::current().id(),
                target
            );
            let condvar = proc.child_exit.clone();
            condvar.wait(proc);
        }
    }

    /// Replaces the current ** process ** with a new process image
    ///
    /// `argv` is an array of argument strings passed to the new program.
    /// `envp` is an array of strings, conventionally of the form `key=value`,
    /// which are passed as environment to the new program.
    ///
    /// NOTICE: `argv` & `envp` can not be NULL (different from Linux)
    ///
    /// NOTICE: for multi-thread programs
    /// A call to any exec function from a process with more than one thread
    /// shall result in all threads being terminated and the new executable image
    /// being loaded and executed.
    pub fn sys_exec(
        &mut self,
        path: *const u8,
        argv: *const *const u8,
        envp: *const *const u8,
    ) -> SysResult {
        info!(
            "exec:BEG: path: {:?}, argv: {:?}, envp: {:?}",
            path, argv, envp
        );
        let mut proc = self.process();
        let path = check_and_clone_cstr(path)?;
        let args = check_and_clone_cstr_array(argv)?;
        let envs = check_and_clone_cstr_array(envp)?;

        if args.is_empty() {
            error!("exec: args is null");
            return Err(SysError::EINVAL);
        }

        info!(
            "exec:STEP2: path: {:?}, args: {:?}, envs: {:?}",
            path, args, envs
        );

        // Kill other threads
        proc.threads.retain(|&tid| {
            if tid != thread::current().id() {
                thread_manager().exit(tid, 1);
            }
            tid == thread::current().id()
        });

        // Read program file
        let inode = proc.lookup_inode(&path)?;

        // Make new Thread
        let (mut vm, entry_addr, ustack_top) =
            Thread::new_user_vm(&inode, &path, args, envs).map_err(|_| SysError::EINVAL)?;

        // close file that FD_CLOEXEC is set
        let close_fds = proc
            .files
            .iter()
            .filter_map(|(fd, file_like)| {
                if let FileLike::File(file) = file_like {
                    if file.fd_cloexec {
                        Some(*fd)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for fd in close_fds {
            proc.files.remove(&fd);
        }

        // Activate new page table
        core::mem::swap(&mut *self.vm(), &mut vm);
        unsafe {
            self.vm().activate();
        }

        // Modify exec path
        proc.exec_path = path.clone();
        drop(proc);

        // Modify the TrapFrame
        *self.tf = TrapFrame::new_user_thread(entry_addr, ustack_top);

        info!("exec:END: path: {:?}", path);
        Ok(0)
    }

    pub fn sys_yield(&mut self) -> SysResult {
        thread::yield_now();
        Ok(0)
    }

    /// Get the current process id
    pub fn sys_getpid(&mut self) -> SysResult {
        info!("getpid");
        Ok(self.process().pid.get())
    }

    pub fn sys_getpgid(&self, mut pid: usize) -> SysResult {
        if pid == 0 {
            pid = self.process().pid.get();
        }
        info!("getpgid: get pgid of process {}", pid);
        let process_table = PROCESSES.read();
        // let process_table: BTreeMap<usize, Weak<Mutex<Process>>> = BTreeMap::new();
        let proc = process_table.get(&pid);
        if let Some(proc) = proc {
            let lock = proc.upgrade().unwrap();
            let proc = lock.lock();
            Ok(proc.pgid as usize)
        } else {
            Err(ESRCH)
        }
    }

    pub fn sys_setpgid(&self, mut pid: usize, pgid: usize) -> SysResult {
        if pid == 0 {
            pid = self.process().pid.get();
        }
        info!("setpgid: set pgid of process {} to {}", pid, pgid);
        let process_table = PROCESSES.read();
        let proc = process_table.get(&pid);
        if let Some(proc) = proc {
            // TODO: check process pid is the child of calling process
            if let Some(proc) = proc.upgrade() {
                let mut proc = proc.lock();
                proc.pgid = pgid as i32;
            }
            Ok(0)
        } else {
            Err(ESRCH)
        }
    }

    /// Get the current thread id
    pub fn sys_gettid(&mut self) -> SysResult {
        info!("gettid");
        // use pid as tid for now
        Ok(thread::current().id())
    }

    /// Get the parent process id
    pub fn sys_getppid(&mut self) -> SysResult {
        info!("getppid");
        let (pid, parent) = self.process().parent.clone();
        if parent.upgrade().is_some() {
            Ok(pid.get())
        } else {
            Ok(0)
        }
    }

    /// Exit the current thread
    pub fn sys_exit(&mut self, exit_code: usize) -> ! {
        let tid = thread::current().id();
        info!("exit: {}, code: {}", tid, exit_code);
        let mut proc = self.process();
        proc.threads.retain(|&id| id != tid);

        // for last thread, exit the process
        if proc.threads.len() == 0 {
            proc.exit(exit_code);
        }

        // perform futex wake 1
        // ref: http://man7.org/linux/man-pages/man2/set_tid_address.2.html
        // FIXME: do it in all possible ways a thread can exit
        //        it has memory access so we can't move it to Thread::drop?
        let clear_child_tid = self.thread.clear_child_tid as *mut u32;
        if !clear_child_tid.is_null() {
            info!("exit: futex {:#?} wake 1", clear_child_tid);
            if let Ok(clear_child_tid_ref) = unsafe { self.vm().check_write_ptr(clear_child_tid) } {
                *clear_child_tid_ref = 0;
                let queue = proc.get_futex(clear_child_tid as usize);
                queue.notify_one();
            }
        }

        drop(proc);

        thread_manager().exit(tid, exit_code as usize);
        thread::yield_now();
        unreachable!();
    }

    /// Exit the current thread group (i.e. process)
    pub fn sys_exit_group(&mut self, exit_code: usize) -> ! {
        let mut proc = self.process();
        info!("exit_group: {}, code: {}", proc.pid, exit_code);

        proc.exit(exit_code);
        drop(proc);
        thread::yield_now();
        unreachable!();
    }

    pub fn sys_nanosleep(&mut self, req: *const TimeSpec) -> SysResult {
        let time = unsafe { *self.vm().check_read_ptr(req)? };
        info!("nanosleep: time: {:#?}", time);
        if !time.is_zero() {
            // TODO: handle spurious wakeup
            thread::sleep(time.to_duration());
            if has_signal_to_do() {
                return Err(EINTR);
            }
        }
        Ok(0)
    }

    pub fn sys_set_priority(&mut self, priority: usize) -> SysResult {
        let pid = thread::current().id();
        thread_manager().set_priority(pid, priority as u8);
        Ok(0)
    }

    pub fn sys_set_tid_address(&mut self, tidptr: *mut u32) -> SysResult {
        info!("set_tid_address: {:?}", tidptr);
        self.thread.clear_child_tid = tidptr as usize;
        Ok(thread::current().id())
    }
}

bitflags! {
    pub struct CloneFlags: usize {
        const CSIGNAL =         0x000000ff;
        const VM =              0x00000100;
        const FS =              0x00000200;
        const FILES =           0x00000400;
        const SIGHAND =         0x00000800;
        const PTRACE =          0x00002000;
        const VFORK =           0x00004000;
        const PARENT =          0x00008000;
        const THREAD =          0x00010000;
        const NEWNS	 =          0x00020000;
        const SYSVSEM =         0x00040000;
        const SETTLS =          0x00080000;
        const PARENT_SETTID =   0x00100000;
        const CHILD_CLEARTID =  0x00200000;
        const DETACHED =        0x00400000;
        const UNTRACED =        0x00800000;
        const CHILD_SETTID =    0x01000000;
        const NEWCGROUP =       0x02000000;
        const NEWUTS =          0x04000000;
        const NEWIPC =          0x08000000;
        const NEWUSER =         0x10000000;
        const NEWPID =          0x20000000;
        const NEWNET =          0x40000000;
        const IO =              0x80000000;
    }
}
