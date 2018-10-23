use alloc::boxed::Box;
use alloc::sync::Arc;
use spin::Mutex;
use scheduler::Scheduler;
use core::cell::UnsafeCell;

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
    Waiting(Pid),
    Sleeping,
    Exited(ExitCode),
}

pub trait Context {
    unsafe fn switch_to(&mut self, target: &mut Context);
}

pub struct ProcessManager {
    procs: [Mutex<Option<Process>>; MAX_PROC_NUM],
    scheduler: Mutex<Box<Scheduler>>,
}

impl ProcessManager {
    pub fn new(scheduler: Box<Scheduler>) -> Self {
        ProcessManager {
            procs: Default::default(),
            scheduler: Mutex::new(scheduler),
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
        // TODO: check parent
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
        self.scheduler.lock().tick(pid)
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
        if proc.status == Status::Ready {
            self.scheduler.lock().insert(pid);
        }
    }

    /// Switch the status of a process.
    /// Insert/Remove it to/from scheduler if necessary.
    pub fn set_status(&self, pid: Pid, status: Status) {
        let mut scheduler = self.scheduler.lock();
        let mut proc_lock = self.procs[pid].lock();
        let mut proc = proc_lock.as_mut().unwrap();
        match (&proc.status, &status) {
            (Status::Ready, Status::Ready) => return,
            (Status::Ready, _) => scheduler.remove(pid),
            (Status::Running(_), _) => {},
            (_, Status::Ready) => scheduler.insert(pid),
            _ => {}
        }
        trace!("process {} {:?} -> {:?}", pid, proc.status, status);
        match proc.status {
            Status::Running(_) => proc.status_after_stop = status,
            _ => proc.status = status,
        }
    }
}
