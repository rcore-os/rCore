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

    pub fn add(&mut self, mut process: Process) {
        let pid = self.alloc_pid();
        process.pid = pid;
        self.procs.insert(pid, process);
    }

    pub fn schedule(&mut self, rsp: &mut usize) {
        let pid = self.find_next();
        self.switch_to(pid, rsp);
    }

    fn find_next(&self) -> Pid {
        *self.procs.keys()
            .find(|&&i| i > self.current_pid
                && self.get(i).status != Status::Exited)
            .unwrap_or(self.procs.keys().nth(0).unwrap())
    }

    fn switch_to(&mut self, pid: Pid, rsp: &mut usize) {
        // for debug print
        let pid0 = self.current_pid;
        let rsp0 = *rsp;

        if pid == self.current_pid {
            return;
        }
        {
            let current = self.procs.get_mut(&self.current_pid).unwrap();
            current.status = Status::Ready;
            current.rsp = *rsp;
        }
        {
            let process = self.procs.get_mut(&pid).unwrap();
            process.status = Status::Running;
            *rsp = process.rsp;

            // switch page table
            if let Some(page_table) = process.page_table.take() {
                let mut active_table = unsafe { ActivePageTable::new() };
                let old_table = active_table.switch(page_table);
                process.page_table = Some(old_table);
            }
        }
        self.current_pid = pid;
        debug!("Processor: switch from {} to {}\n  rsp: {:#x} -> {:#x}", pid0, pid, rsp0, rsp);
    }

    fn get(&self, pid: Pid) -> &Process {
        self.procs.get(&pid).unwrap()
    }

    pub fn current(&self) -> &Process {
        self.get(self.current_pid)
    }

    pub fn kill(&mut self, pid: Pid) {
        let process = self.procs.get_mut(&pid).unwrap();
        process.status = Status::Exited;
        // TODO: Remove process from set
    }

    pub fn exit(&mut self, pid: Pid, error_code: usize) {
        debug!("Processor: {} exit, code: {}", pid, error_code);
        self.kill(pid);
    }
}

impl Debug for Processor {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_map()
            .entries(self.procs.iter().map(|(pid, proc0)| { (pid, &proc0.name) }))
            .finish()
    }
}