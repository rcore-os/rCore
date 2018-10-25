use alloc::boxed::Box;
use alloc::sync::Arc;
use spin::Mutex;
use scheduler::Scheduler;
use core::cell::UnsafeCell;
use alloc::vec::Vec;
use event_hub::EventHub;

struct Process {
    id: Pid,
    status: Status,
    status_after_stop: Status,
    context: Option<Box<Context>>,
}

pub type Pid = usize;
type ExitCode = usize;
const MAX_PROC_NUM: usize = 32;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Status {
    Ready,
    Running(usize),
    Sleeping,
    Waiting(Pid),
    /// aka ZOMBIE. Its context was dropped.
    Exited(ExitCode),
}

enum Event {
    Wakeup(Pid),
}

pub trait Context {
    unsafe fn switch_to(&mut self, target: &mut Context);
}

pub struct ProcessManager {
    procs: [Mutex<Option<Process>>; MAX_PROC_NUM],
    scheduler: Mutex<Box<Scheduler>>,
    wait_queue: [Mutex<Vec<Pid>>; MAX_PROC_NUM],
    event_hub: Mutex<EventHub<Event>>,
}

impl ProcessManager {
    pub fn new(scheduler: Box<Scheduler>) -> Self {
        ProcessManager {
            procs: Default::default(),
            scheduler: Mutex::new(scheduler),
            wait_queue: Default::default(),
            event_hub: Mutex::new(EventHub::new()),
        }
    }

    fn alloc_pid(&self) -> Pid {
        for i in 0..MAX_PROC_NUM {
            if self.procs[i].lock().is_none() {
                return i;
            }
        }
        panic!("Process number exceeded");
    }

    /// Add a new process
    pub fn add(&self, context: Box<Context>) -> Pid {
        let pid = self.alloc_pid();
        *self.procs[pid].lock() = Some(Process {
            id: pid,
            status: Status::Ready,
            status_after_stop: Status::Ready,
            context: Some(context),
        });
        self.scheduler.lock().insert(pid);
        pid
    }

    /// Make process `pid` time slice -= 1.
    /// Return true if time slice == 0.
    /// Called by timer interrupt handler.
    pub fn tick(&self, pid: Pid) -> bool {
        let mut event_hub = self.event_hub.lock();
        event_hub.tick();
        while let Some(event) = event_hub.pop() {
            match event {
                Event::Wakeup(pid) => self.set_status(pid, Status::Ready),
            }
        }
        self.scheduler.lock().tick(pid)
    }

    /// Set the priority of process `pid`
    pub fn set_priority(&self, pid: Pid, priority: u8) {
        self.scheduler.lock().set_priority(pid, priority);
    }

    /// Called by Processor to get a process to run.
    /// The manager first mark it `Running`,
    /// then take out and return its Context.
    pub fn run(&self, cpu_id: usize) -> (Pid, Box<Context>) {
        let mut scheduler = self.scheduler.lock();
        let pid = scheduler.select()
            .expect("failed to select a runnable process");
        scheduler.remove(pid);
        let mut proc_lock = self.procs[pid].lock();
        let mut proc = proc_lock.as_mut().unwrap();
        proc.status = Status::Running(cpu_id);
        (pid, proc.context.take().unwrap())
    }

    /// Called by Processor to finish running a process
    /// and give its context back.
    pub fn stop(&self, pid: Pid, context: Box<Context>) {
        let mut proc_lock = self.procs[pid].lock();
        let mut proc = proc_lock.as_mut().unwrap();
        proc.status = proc.status_after_stop.clone();
        proc.status_after_stop = Status::Ready;
        proc.context = Some(context);
        match proc.status {
            Status::Ready => self.scheduler.lock().insert(pid),
            Status::Exited(_) => proc.context = None,
            _ => {}
        }
    }

    /// Switch the status of a process.
    /// Insert/Remove it to/from scheduler if necessary.
    fn set_status(&self, pid: Pid, status: Status) {
        let mut scheduler = self.scheduler.lock();
        let mut proc_lock = self.procs[pid].lock();
        let mut proc = proc_lock.as_mut().unwrap();
        trace!("process {} {:?} -> {:?}", pid, proc.status, status);
        match (&proc.status, &status) {
            (Status::Ready, Status::Ready) => return,
            (Status::Ready, _) => scheduler.remove(pid),
            (Status::Running(_), _) => {},
            (Status::Exited(_), _) => panic!("can not set status for a exited process"),
            (Status::Waiting(target), Status::Exited(_)) =>
                self.wait_queue[*target].lock().retain(|&i| i != pid),
            // TODO: Sleep -> Exited  Remove wakeup event.
            (_, Status::Ready) => scheduler.insert(pid),
            _ => {}
        }
        match proc.status {
            Status::Running(_) => proc.status_after_stop = status,
            _ => proc.status = status,
        }
        match proc.status {
            Status::Exited(_) => proc.context = None,
            _ => {}
        }
    }


    pub fn get_status(&self, pid: Pid) -> Option<Status> {
        self.procs[pid].lock().as_ref().map(|p| p.status.clone())
    }

    pub fn remove(&self, pid: Pid) {
        let mut proc_lock = self.procs[pid].lock();
        let proc = proc_lock.as_ref().unwrap();
        match proc.status {
            Status::Exited(_) => *proc_lock = None,
            _ => panic!("can not remove non-exited process"),
        }
    }

    pub fn sleep(&self, pid: Pid, time: usize) {
        self.set_status(pid, Status::Sleeping);
        if time != 0 {
            self.event_hub.lock().push(time, Event::Wakeup(pid));
        }
    }

    pub fn wakeup(&self, pid: Pid) {
        self.set_status(pid, Status::Ready);
    }

    pub fn wait(&self, pid: Pid, target: Pid) {
        self.set_status(pid, Status::Waiting(target));
        self.wait_queue[target].lock().push(pid);
    }

    pub fn exit(&self, pid: Pid, code: ExitCode) {
        self.set_status(pid, Status::Exited(code));
        for waiter in self.wait_queue[pid].lock().drain(..) {
            self.wakeup(waiter);
        }
    }
}
