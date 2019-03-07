use alloc::boxed::Box;
use alloc::vec::Vec;
use spin::Mutex;
use log::*;
use crate::scheduler::Scheduler;
use crate::timer::Timer;

struct Thread {
    status: Status,
    status_after_stop: Status,
    context: Option<Box<Context>>,
    parent: Tid,
    children: Vec<Tid>,
}

pub type Tid = usize;
type ExitCode = usize;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Status {
    Ready,
    Running(usize),
    Sleeping,
    Waiting(Tid),
    /// aka ZOMBIE. Its context was dropped.
    Exited(ExitCode),
}

#[derive(Eq, PartialEq)]
enum Event {
    Wakeup(Tid),
}

pub trait Context {
    unsafe fn switch_to(&mut self, target: &mut Context);
}

pub struct ThreadPool {
    threads: Vec<Mutex<Option<Thread>>>,
    scheduler: Box<Scheduler>,
    timer: Mutex<Timer<Event>>,
}

impl ThreadPool {
    pub fn new(scheduler: impl Scheduler, max_proc_num: usize) -> Self {
        ThreadPool {
            threads: new_vec_default(max_proc_num),
            scheduler: Box::new(scheduler),
            timer: Mutex::new(Timer::new()),
        }
    }

    fn alloc_tid(&self) -> Tid {
        for (i, proc) in self.threads.iter().enumerate() {
            if proc.lock().is_none() {
                return i;
            }
        }
        panic!("Process number exceeded");
    }

    /// Add a new process
    pub fn add(&self, context: Box<Context>, parent: Tid) -> Tid {
        let tid = self.alloc_tid();
        *(&self.threads[tid]).lock() = Some(Thread {
            status: Status::Ready,
            status_after_stop: Status::Ready,
            context: Some(context),
            parent,
            children: Vec::new(),
        });
        self.scheduler.push(tid);
        self.threads[parent].lock().as_mut().expect("invalid parent proc")
            .children.push(tid);
        tid
    }

    /// Make process `tid` time slice -= 1.
    /// Return true if time slice == 0.
    /// Called by timer interrupt handler.
    pub(crate) fn tick(&self, cpu_id: usize, tid: Option<Tid>) -> bool {
        if cpu_id == 0 {
            let mut timer = self.timer.lock();
            timer.tick();
            while let Some(event) = timer.pop() {
                match event {
                    Event::Wakeup(tid) => self.set_status(tid, Status::Ready),
                }
            }
        }
        match tid {
            Some(tid) => self.scheduler.tick(tid),
            None => false,
        }
    }

    /// Set the priority of process `tid`
    pub fn set_priority(&self, tid: Tid, priority: u8) {
        self.scheduler.set_priority(tid, priority);
    }

    /// Called by Processor to get a process to run.
    /// The manager first mark it `Running`,
    /// then take out and return its Context.
    pub(crate) fn run(&self, cpu_id: usize) -> Option<(Tid, Box<Context>)> {
        self.scheduler.pop(cpu_id)
            .map(|tid| {
                let mut proc_lock = self.threads[tid].lock();
                let mut proc = proc_lock.as_mut().expect("process not exist");
                proc.status = Status::Running(cpu_id);
                (tid, proc.context.take().expect("context not exist"))
            })
    }

    /// Called by Processor to finish running a process
    /// and give its context back.
    pub(crate) fn stop(&self, tid: Tid, context: Box<Context>) {
        let mut proc_lock = self.threads[tid].lock();
        let mut proc = proc_lock.as_mut().expect("process not exist");
        proc.status = proc.status_after_stop.clone();
        proc.status_after_stop = Status::Ready;
        proc.context = Some(context);
        match proc.status {
            Status::Ready => self.scheduler.push(tid),
            Status::Exited(_) => self.exit_handler(tid, proc),
            _ => {}
        }
    }

    /// Switch the status of a process.
    /// Insert/Remove it to/from scheduler if necessary.
    fn set_status(&self, tid: Tid, status: Status) {
        let mut proc_lock = self.threads[tid].lock();
        if let Some(mut proc) = proc_lock.as_mut() {
            trace!("process {} {:?} -> {:?}", tid, proc.status, status);
            match (&proc.status, &status) {
                (Status::Ready, Status::Ready) => return,
                (Status::Ready, _) => panic!("can not remove a process from ready queue"),
                (Status::Exited(_), _) => panic!("can not set status for a exited process"),
                (Status::Sleeping, Status::Exited(_)) => self.timer.lock().stop(Event::Wakeup(tid)),
                (Status::Running(_), Status::Ready) => {} // process will be added to scheduler in stop() 
                (_, Status::Ready) => self.scheduler.push(tid),
                _ => {}
            }
            match proc.status {
                Status::Running(_) => proc.status_after_stop = status,
                _ => proc.status = status,
            }
            match proc.status {
                Status::Exited(_) => self.exit_handler(tid, proc),
                _ => {}
            }
        }
    }

    pub fn get_status(&self, tid: Tid) -> Option<Status> {
        if tid < self.threads.len() {
            self.threads[tid].lock().as_ref().map(|p| p.status.clone())
        } else {
            None
        }
    }

    /// Remove an exited proc `tid`.
    /// Its all children will be set parent to 0.
    pub fn remove(&self, tid: Tid) {
        let mut proc_lock = self.threads[tid].lock();
        let proc = proc_lock.as_ref().expect("process not exist");
        match proc.status {
            Status::Exited(_) => {}
            _ => panic!("can not remove non-exited process"),
        }
        // orphan procs
        for child in proc.children.iter() {
            (&self.threads[*child]).lock().as_mut().expect("process not exist").parent = 0;
        }
        // remove self from parent's children list
        self.threads[proc.parent].lock().as_mut().expect("process not exist")
            .children.retain(|&i| i != tid);
        // release the tid
        *proc_lock = None;
    }

    /// Sleep `tid` for `time` ticks.
    /// `time` == 0 means sleep forever
    pub fn sleep(&self, tid: Tid, time: usize) {
        self.set_status(tid, Status::Sleeping);
        if time != 0 {
            self.timer.lock().start(time, Event::Wakeup(tid));
        }
    }

    pub fn wakeup(&self, tid: Tid) {
        self.set_status(tid, Status::Ready);
    }

    pub fn wait(&self, tid: Tid, target: Tid) {
        self.set_status(tid, Status::Waiting(target));
    }
    pub fn wait_child(&self, tid: Tid) {
        self.set_status(tid, Status::Waiting(0));
    }

    pub fn get_children(&self, tid: Tid) -> Vec<Tid> {
        self.threads[tid].lock().as_ref().expect("process not exist").children.clone()
    }
    pub fn get_parent(&self, tid: Tid) -> Tid {
        self.threads[tid].lock().as_ref().expect("process not exist").parent
    }

    pub fn exit(&self, tid: Tid, code: ExitCode) {
        // NOTE: if `tid` is running, status change will be deferred.
        self.set_status(tid, Status::Exited(code));
    }
    /// Called when a process exit
    fn exit_handler(&self, tid: Tid, proc: &mut Thread) {
        // wakeup parent if waiting
        let parent = proc.parent;
        match self.get_status(parent).expect("process not exist") {
            Status::Waiting(target) if target == tid || target == 0 => self.wakeup(parent),
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
