use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;
use log::*;
use core::cell::UnsafeCell;
use crate::scheduler::Scheduler;
use crate::event_hub::EventHub;

struct Process {
    id: Pid,
    status: Status,
    status_after_stop: Status,
    context: Option<Box<Context>>,
    parent: Pid,
    children: Vec<Pid>,
}

pub type Pid = usize;
type ExitCode = usize;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Status {
    Ready,
    Running(usize),
    Sleeping,
    Waiting(Pid),
    /// aka ZOMBIE. Its context was dropped.
    Exited(ExitCode),
}

#[derive(Eq, PartialEq)]
enum Event {
    Wakeup(Pid),
}

pub trait Context {
    unsafe fn switch_to(&mut self, target: &mut Context);
}

pub struct ProcessManager {
    procs: Vec<Mutex<Option<Process>>>,
    scheduler: Mutex<Box<Scheduler>>,
    event_hub: Mutex<EventHub<Event>>,
}

impl ProcessManager {
    pub fn new(scheduler: Box<Scheduler>, max_proc_num: usize) -> Self {
        ProcessManager {
            procs: new_vec_default(max_proc_num),
            scheduler: Mutex::new(scheduler),
            event_hub: Mutex::new(EventHub::new()),
        }
    }

    fn alloc_pid(&self) -> Pid {
        for (i, proc) in self.procs.iter().enumerate() {
            if proc.lock().is_none() {
                return i;
            }
        }
        panic!("Process number exceeded");
    }

    /// Add a new process
    pub fn add(&self, context: Box<Context>, parent: Pid) -> Pid {
        let pid = self.alloc_pid();
        *(&self.procs[pid]).lock() = Some(Process {
            id: pid,
            status: Status::Ready,
            status_after_stop: Status::Ready,
            context: Some(context),
            parent,
            children: Vec::new(),
        });
        self.scheduler.lock().insert(pid);
        self.procs[parent].lock().as_mut().expect("invalid parent proc")
            .children.push(pid);
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
        let mut proc = proc_lock.as_mut().expect("process not exist");
        proc.status = Status::Running(cpu_id);
        (pid, proc.context.take().expect("context not exist"))
    }

    /// Called by Processor to finish running a process
    /// and give its context back.
    pub fn stop(&self, pid: Pid, context: Box<Context>) {
        let mut proc_lock = self.procs[pid].lock();
        let mut proc = proc_lock.as_mut().expect("process not exist");
        proc.status = proc.status_after_stop.clone();
        proc.status_after_stop = Status::Ready;
        proc.context = Some(context);
        match proc.status {
            Status::Ready => self.scheduler.lock().insert(pid),
            Status::Exited(_) => self.exit_handler(pid, proc),
            _ => {}
        }
    }

    /// Switch the status of a process.
    /// Insert/Remove it to/from scheduler if necessary.
    fn set_status(&self, pid: Pid, status: Status) {
        let mut proc_lock = self.procs[pid].lock();
        let mut proc = proc_lock.as_mut().expect("process not exist");
        trace!("process {} {:?} -> {:?}", pid, proc.status, status);
        match (&proc.status, &status) {
            (Status::Ready, Status::Ready) => return,
            (Status::Ready, _) => self.scheduler.lock().remove(pid),
            (Status::Exited(_), _) => panic!("can not set status for a exited process"),
            (Status::Sleeping, Status::Exited(_)) => self.event_hub.lock().remove(Event::Wakeup(pid)),
            (_, Status::Ready) => self.scheduler.lock().insert(pid),
            _ => {}
        }
        match proc.status {
            Status::Running(_) => proc.status_after_stop = status,
            _ => proc.status = status,
        }
        match proc.status {
            Status::Exited(_) => self.exit_handler(pid, proc),
            _ => {}
        }
    }

    pub fn get_status(&self, pid: Pid) -> Option<Status> {
        self.procs[pid].lock().as_ref().map(|p| p.status.clone())
    }

    /// Remove an exited proc `pid`.
    /// Its all children will be set parent to 0.
    pub fn remove(&self, pid: Pid) {
        let mut proc_lock = self.procs[pid].lock();
        let proc = proc_lock.as_ref().expect("process not exist");
        match proc.status {
            Status::Exited(_) => {}
            _ => panic!("can not remove non-exited process"),
        }
        // orphan procs
        for child in proc.children.iter() {
            (&self.procs[*child]).lock().as_mut().expect("process not exist").parent = 0;
        }
        // remove self from parent's children list
        self.procs[proc.parent].lock().as_mut().expect("process not exist")
            .children.retain(|&i| i != pid);
        // release the pid
        *proc_lock = None;
    }

    /// Sleep `pid` for `time` ticks.
    /// `time` == 0 means sleep forever
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
    }
    pub fn wait_child(&self, pid: Pid) {
        self.set_status(pid, Status::Waiting(0));
    }

    pub fn get_children(&self, pid: Pid) -> Vec<Pid> {
        self.procs[pid].lock().as_ref().expect("process not exist").children.clone()
    }

    pub fn exit(&self, pid: Pid, code: ExitCode) {
        // NOTE: if `pid` is running, status change will be deferred.
        self.set_status(pid, Status::Exited(code));
    }
    /// Called when a process exit
    fn exit_handler(&self, pid: Pid, proc: &mut Process) {
        // wakeup parent if waiting
        let parent = proc.parent;
        match self.get_status(parent).expect("process not exist") {
            Status::Waiting(target) if target == pid || target == 0 => self.wakeup(parent),
            _ => {}
        }
        // drop its context
        proc.context = None;
    }
}

fn new_vec_default<T: Default>(size: usize) -> Vec<T> {
    let mut vec = Vec::new();
    vec.resize_default(size);
    vec
}
