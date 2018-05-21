use alloc::BTreeMap;
use memory::{ActivePageTable, InactivePageTable};
use super::process::*;
use core::cell::RefCell;
use core::fmt::{Debug, Formatter, Error};
use util::{EventHub, GetMut2};

pub struct Processor {
    procs: BTreeMap<Pid, Process>,
    current_pid: Pid,
    event_hub: EventHub<Event>,
    /// All kernel threads share one page table.
    /// When running user process, it will be stored here.
    kernel_page_table: Option<InactivePageTable>,
    /// Choose what on next schedule ?
    next: Option<Pid>,
}

impl Processor {
    pub fn new() -> Self {
        Processor {
            procs: BTreeMap::<Pid, Process>::new(),
            current_pid: 0,
            event_hub: {
                let mut e = EventHub::new();
                e.push(10, Event::Schedule);
                e
            },
            kernel_page_table: None,
            next: None,
        }
    }

    pub fn set_reschedule(&mut self) {
        let pid = self.current_pid;
        self.get_mut(pid).status = Status::Ready;
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

    /// Called by timer.
    /// Handle events.
    pub fn tick(&mut self) {
        self.event_hub.tick();
        while let Some(event) = self.event_hub.pop() {
            debug!("Processor: event {:?}", event);
            match event {
                Event::Schedule => {
                    self.event_hub.push(10, Event::Schedule);
                    self.set_reschedule();
                },
                Event::Wakeup(pid) => {
                    self.get_mut(pid).status = Status::Ready;
                    self.set_reschedule();
                    self.next = Some(pid);
                },
            }
        }
    }

    pub fn get_time(&self) -> usize {
        self.event_hub.get_time()
    }

    pub fn add(&mut self, mut process: Process) -> Pid {
        let pid = self.alloc_pid();
        process.pid = pid;
        self.procs.insert(pid, process);
        pid
    }

    /// Called every interrupt end
    /// Do schedule ONLY IF current status != Running
    pub fn schedule(&mut self, rsp: &mut usize) {
        if self.current().status == Status::Running {
            return;
        }
        let pid = self.next.take().unwrap_or_else(|| self.find_next());
        self.switch_to(pid, rsp);
    }

    fn find_next(&self) -> Pid {
        *self.procs.keys()
            .find(|&&i| i > self.current_pid
                && self.get(i).status == Status::Ready)
            .unwrap_or(self.procs.keys().next().unwrap())
    }

    /// Switch process to `pid`, switch page table if necessary.
    /// Store `rsp` and point it to target kernel stack.
    /// The current status must be set before, and not be `Running`.
    fn switch_to(&mut self, pid: Pid, rsp: &mut usize) {
        // for debug print
        let pid0 = self.current_pid;
        let rsp0 = *rsp;

        if pid == self.current_pid {
            return;
        }
        self.current_pid = pid;

        let (from, to) = self.procs.get_mut2(pid0, pid);

        // set `from`
        assert_ne!(from.status, Status::Running);
        from.rsp = *rsp;

        // set `to`
        assert_eq!(to.status, Status::Ready);
        to.status = Status::Running;
        *rsp = to.rsp;

        // switch page table
        if from.is_user || to.is_user {
            let (from_pt, to_pt) = match (from.is_user, to.is_user) {
                (true, true) => (&mut from.page_table, &mut to.page_table),
                (true, false) => (&mut from.page_table, &mut self.kernel_page_table),
                (false, true) => (&mut self.kernel_page_table, &mut to.page_table),
                _ => panic!(),
            };
            assert!(from_pt.is_none());
            assert!(to_pt.is_some());
            let mut active_table = unsafe { ActivePageTable::new() };
            let old_table = active_table.switch(to_pt.take().unwrap());
            *from_pt = Some(old_table);
        }

        info!("Processor: switch from {} to {}\n  rsp: {:#x} -> {:#x}", pid0, pid, rsp0, rsp);
    }

    fn get(&self, pid: Pid) -> &Process {
        self.procs.get(&pid).unwrap()
    }
    fn get_mut(&mut self, pid: Pid) -> &mut Process {
        self.procs.get_mut(&pid).unwrap()
    }
    pub fn current(&self) -> &Process {
        self.get(self.current_pid)
    }
    pub fn current_pid(&self) -> Pid {
        self.current_pid
    }

    pub fn kill(&mut self, pid: Pid) {
        self.exit(pid, 0x1000); // TODO: error code for killed
    }

    pub fn exit(&mut self, pid: Pid, error_code: ErrorCode) {
        info!("Processor: {} exit, code: {}", pid, error_code);
        self.get_mut(pid).status = Status::Exited(error_code);
        if let Some(waiter) = self.find_waiter(pid) {
            {
                let p = self.get_mut(waiter);
                p.status = Status::Ready;
                p.set_return_value(error_code);
            }
            // FIXME: remove this process
            self.get_mut(pid).parent = 0;
//            info!("Processor: remove {}", pid);
//            self.procs.remove(&pid);
        }
    }

    pub fn sleep(&mut self, pid: Pid, time: usize) {
        self.get_mut(pid).status = Status::Sleeping;
        self.event_hub.push(time, Event::Wakeup(pid));
    }

    /// Let current process wait for another
    pub fn current_wait_for(&mut self, target: WaitTarget) -> WaitResult {
        info!("Processor: current {} wait for {:?}", self.current_pid, target);
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
            WaitResult::Ok(pid, exit_code)
        } else {
            info!("Processor: {} wait for {}", self.current_pid, pid);
            let current_pid = self.current_pid;
            self.get_mut(current_pid).status = Status::Waiting(pid);
            WaitResult::Blocked
        }
    }

    fn find_waiter(&self, pid: Pid) -> Option<Pid> {
        self.procs.values().find(|&p| {
            p.status == Status::Waiting(pid) ||
                (p.status == Status::Waiting(0) && self.get(pid).parent == p.pid)
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

#[derive(Debug)]
pub enum WaitTarget {
    AnyChild,
    Proc(Pid),
}

#[derive(Debug)]
pub enum WaitResult {
    /// The target process is still running.
    /// The waiter's status will be set to `Waiting`.
    Blocked,
    /// The target process is exited with `ErrorCode`.
    Ok(Pid, ErrorCode),
    /// The target process is not exist.
    NotExist,
}

#[derive(Debug)]
enum Event {
    Schedule,
    Wakeup(Pid),
}

impl GetMut2<Pid> for BTreeMap<Pid, Process> {
    type Output = Process;
    fn get_mut(&mut self, id: Pid) -> &mut Process {
        self.get_mut(&id).unwrap()
    }
}