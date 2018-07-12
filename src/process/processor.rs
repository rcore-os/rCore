use alloc::BTreeMap;
use core::fmt::{Debug, Error, Formatter};
use super::process::*;
use super::scheduler::*;
use util::{EventHub, GetMut2};

pub struct Processor {
    procs: BTreeMap<Pid, Process>,
    current_pid: Pid,
    event_hub: EventHub<Event>,
    /// Choose what on next schedule ?
    next: Option<Pid>,
    // WARNING: if MAX_PROCESS_NUM is too large, will cause stack overflow
    scheduler: RRScheduler,
}

// TODO: 除schedule()外的其它函数，应该只设置进程状态，不应调用schedule
impl Processor {
    pub fn new() -> Self {
        Processor {
            procs: BTreeMap::<Pid, Process>::new(),
            current_pid: 0,
            event_hub: EventHub::new(),
            next: None,
            // NOTE: max_time_slice <= 5 to ensure 'priority' test pass
            scheduler: RRScheduler::new(5),
        }
    }

    pub fn lab6_set_priority(&mut self, priority: u8) {
        unimplemented!();
//        self.scheduler.set_priority(self.current_pid, priority);
    }

    pub fn set_reschedule(&mut self) {
        let pid = self.current_pid;
        self.set_status(pid, Status::Ready);
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

    fn set_status(&mut self, pid: Pid, status: Status) {
        let status0 = self.get(pid).status.clone();
        match (&status0, &status) {
            (&Status::Ready, &Status::Ready) => return,
            (&Status::Ready, _) => self.scheduler.remove(pid),
            (_, &Status::Ready) => self.scheduler.insert(pid),
            _ => {}
        }
        trace!("process {} {:?} -> {:?}", pid, status0, status);
        self.get_mut(pid).status = status;
    }

    /// Called by timer.
    /// Handle events.
    pub fn tick(&mut self) {
        let current_pid = self.current_pid;
        if self.scheduler.tick(current_pid) {
            self.set_reschedule();
        }
        self.event_hub.tick();
        while let Some(event) = self.event_hub.pop() {
            debug!("event {:?}", event);
            match event {
                Event::Schedule => {
                    self.event_hub.push(10, Event::Schedule);
                    self.set_reschedule();
                },
                Event::Wakeup(pid) => {
                    self.set_status(pid, Status::Ready);
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
        if process.status == Status::Ready {
            self.scheduler.insert(pid);
        }
        self.procs.insert(pid, process);
        pid
    }

    /// Called every interrupt end
    /// Do schedule ONLY IF current status != Running
    pub fn schedule(&mut self) {
        if self.current().status == Status::Running {
            return;
        }
        let pid = self.next.take().unwrap_or_else(|| self.scheduler.select().unwrap());
        self.switch_to(pid);
    }

    /// Switch process to `pid`, switch page table if necessary.
    /// Store `rsp` and point it to target kernel stack.
    /// The current status must be set before, and not be `Running`.
    fn switch_to(&mut self, pid: Pid) {
        // for debug print
        let pid0 = self.current_pid;

        if pid == self.current_pid {
            if self.current().status != Status::Running {
                self.set_status(pid, Status::Running);
            }
            return;
        }
        self.current_pid = pid;

        let (from, to) = self.procs.get_mut2(pid0, pid);

        assert_ne!(from.status, Status::Running);
        assert_eq!(to.status, Status::Ready);
        to.status = Status::Running;
        self.scheduler.remove(pid);

        info!("switch from {} to {} {:x?}", pid0, pid, to.context);
        unsafe {
            // FIXME: safely pass MutexGuard
            use core::mem::forget;
            super::PROCESSOR.try().unwrap().force_unlock();
            from.context.switch(&mut to.context);
            forget(super::PROCESSOR.try().unwrap().lock());
        }
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
        info!("{} exit, code: {}", pid, error_code);
        self.set_status(pid, Status::Exited(error_code));
        if let Some(waiter) = self.find_waiter(pid) {
            info!("  then wakeup {}", waiter);
            self.set_status(waiter, Status::Ready);
            self.switch_to(waiter); // yield
        }
    }

    pub fn sleep(&mut self, pid: Pid, time: usize) {
        self.set_status(pid, Status::Sleeping);
        self.event_hub.push(time, Event::Wakeup(pid));
    }
    pub fn sleep_(&mut self, pid: Pid) {
        self.set_status(pid, Status::Sleeping);
    }
    pub fn wakeup_(&mut self, pid: Pid) {
        self.set_status(pid, Status::Ready);
    }

    /// Let current process wait for another
    pub fn current_wait_for(&mut self, pid: Pid) -> WaitResult {
        info!("current {} wait for {:?}", self.current_pid, pid);
        if self.procs.values().filter(|&p| p.parent == self.current_pid).next().is_none() {
            return WaitResult::NotExist;
        }
        let pid = self.try_wait(pid).unwrap_or_else(|| {
            let current_pid = self.current_pid;
            self.set_status(current_pid, Status::Waiting(pid));
            self.schedule(); // yield
            self.try_wait(pid).unwrap()
        });
        let exit_code = self.get(pid).exit_code().unwrap();
        info!("{} wait end and remove {}", self.current_pid, pid);
        self.procs.remove(&pid);
        WaitResult::Ok(pid, exit_code)
    }

    /// Try to find a exited wait target
    fn try_wait(&mut self, pid: Pid) -> Option<Pid> {
        match pid {
            0 => self.procs.values()
                .find(|&p| p.parent == self.current_pid && p.exit_code().is_some())
                .map(|p| p.pid),
            _ => self.get(pid).exit_code().map(|_| pid),
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
pub enum WaitResult {
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