use alloc::BTreeMap;
use memory::{ActivePageTable, InactivePageTable};
use super::*;
use core::cell::RefCell;
use core::fmt::{Debug, Formatter, Error};

pub struct Processor {
    procs: BTreeMap<Pid, Process>,
    current_pid: Pid,
}

impl Processor {
    pub fn new() -> Self {
        Processor {
            procs: BTreeMap::<Pid, Process>::new(),
            current_pid: 0,
        }
    }

    fn alloc_pid(&self) -> Pid {
        let mut next: Pid = 0;
        for &i in self.procs.keys() {
            if i != next {
                return next;
            } else {
                next = i + 1;
            }
        }
        return next;
    }

    pub fn add(&mut self, mut process: Process) -> Pid {
        let pid = self.alloc_pid();
        process.pid = pid;
        self.procs.insert(pid, process);
        pid
    }

    pub fn schedule(&mut self, rsp: &mut usize) {
        let pid = self.find_next();
        self.switch_to(pid, rsp);
    }

    fn find_next(&self) -> Pid {
        *self.procs.keys()
            .find(|&&i| i > self.current_pid
                && self.get(i).exit_code().is_none())
            .unwrap_or(self.procs.keys().next().unwrap())
    }

    /// Switch process to `pid`, switch page table if necessary.
    /// Store `rsp` and point it to target kernel stack.
    /// The current status will be set to `Ready` if it is `Running` now.
    fn switch_to(&mut self, pid: Pid, rsp: &mut usize) {
        // for debug print
        let pid0 = self.current_pid;
        let rsp0 = *rsp;

        if pid == self.current_pid {
            return;
        }
        self.current_pid = pid;

        let (from, to) = self.get_mut2(pid0, pid);

        // set `from`
        if from.status == Status::Running {
            from.status = Status::Ready;
        }
        from.rsp = *rsp;

        // set `to`
        assert_eq!(to.status, Status::Ready);
        to.status = Status::Running;
        *rsp = to.rsp;

        // switch page table
        if let Some(page_table) = to.page_table.take() {
            let mut active_table = unsafe { ActivePageTable::new() };
            let old_table = active_table.switch(page_table);
            from.page_table = Some(old_table);
        }

        info!("Processor: switch from {} to {}\n  rsp: {:#x} -> {:#x}", pid0, pid, rsp0, rsp);
    }

    fn get(&self, pid: Pid) -> &Process {
        self.procs.get(&pid).unwrap()
    }
    fn get_mut(&mut self, pid: Pid) -> &mut Process {
        self.procs.get_mut(&pid).unwrap()
    }
    fn get_mut2(&mut self, pid1: Pid, pid2: Pid) -> (&mut Process, &mut Process) {
        assert_ne!(pid1, pid2);
        let procs1 = &mut self.procs as *mut BTreeMap<_, _>;
        let procs2 = procs1;
        let p1 = unsafe { &mut *procs1 }.get_mut(&pid1).unwrap();
        let p2 = unsafe { &mut *procs2 }.get_mut(&pid2).unwrap();
        (p1, p2)
    }
    pub fn current(&self) -> &Process {
        self.get(self.current_pid)
    }

    pub fn kill(&mut self, pid: Pid) {
        self.exit(pid, 0x1000); // TODO: error code for killed
    }

    pub fn exit(&mut self, pid: Pid, error_code: ErrorCode) {
        assert_ne!(pid, self.current_pid);
        info!("Processor: {} exit, code: {}", pid, error_code);
        self.get_mut(pid).status = Status::Exited(error_code);
        if let Some(waiter) = self.find_waiter(pid) {
            {
                let p = self.get_mut(waiter);
                p.status = Status::Ready;
                p.set_return_value(error_code);
            }
            info!("Processor: remove {}", pid);
            self.procs.remove(&pid);
        }
    }

    /// Let current process wait for another
    pub fn current_wait_for(&mut self, target: WaitTarget) -> WaitResult {
        // Find one target process and it's exit code
        let (pid, exit_code) = match target {
            WaitTarget::AnyChild => {
                let childs = self.procs.values()
                    .filter(|&p| p.parent == self.current_pid);
                if childs.clone().next().is_none() {
                    return WaitResult::NotExist;
                }
                childs.clone()
                    .find(|&p| p.exit_code().is_some())
                    .map(|p| (p.pid, p.exit_code()))
                    .unwrap_or((0, None))
            }
            WaitTarget::Proc(pid) => (pid, self.get(pid).exit_code()),
        };
        if let Some(exit_code) = exit_code {
            info!("Processor: {} wait find and remove {}", self.current_pid, pid);
            self.procs.remove(&pid);
            WaitResult::Ok(exit_code)
        } else {
            info!("Processor: {} wait for {}", self.current_pid, pid);
            let current_pid = self.current_pid;
            self.get_mut(current_pid).status = Status::Sleeping(pid);
            WaitResult::Blocked
        }
    }

    fn find_waiter(&self, pid: Pid) -> Option<Pid> {
        self.procs.values().find(|&p| {
            p.status == Status::Sleeping(pid) ||
                (p.status == Status::Sleeping(0) && self.get(pid).parent == p.pid)
        }).map(|ref p| p.pid)
    }
}

impl Debug for Processor {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_map()
            .entries(self.procs.iter().map(|(pid, proc0)| { (pid, &proc0.name) }))
            .finish()
    }
}

pub enum WaitTarget {
    AnyChild,
    Proc(Pid),
}

pub enum WaitResult {
    /// The target process is still running.
    /// The waiter's status will be set to `Sleeping`.
    Blocked,
    /// The target process is exited with `ErrorCode`.
    Ok(ErrorCode),
    /// The target process is not exist.
    NotExist,
}