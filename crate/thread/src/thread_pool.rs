use crate::scheduler::Scheduler;
use crate::timer::Timer;
use alloc::boxed::Box;
use alloc::vec::Vec;
use log::*;
use spin::{Mutex, MutexGuard};

struct Thread {
    status: Status,
    status_after_stop: Status,
    waiter: Option<Tid>,
    context: Option<Box<Context>>,
}

pub type Tid = usize;
type ExitCode = usize;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Status {
    Ready,
    Running(usize),
    Sleeping,
    /// aka ZOMBIE. Its context was dropped.
    Exited(ExitCode),
}

#[derive(Eq, PartialEq)]
enum Event {
    Wakeup(Tid),
}

pub trait Context {
    /// Switch to target context
    unsafe fn switch_to(&mut self, target: &mut Context);

    /// A tid is allocated for this context
    fn set_tid(&mut self, tid: Tid);
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

    fn alloc_tid(&self) -> (Tid, MutexGuard<Option<Thread>>) {
        for (i, proc) in self.threads.iter().enumerate() {
            let thread = proc.lock();
            if thread.is_none() {
                return (i, thread);
            }
        }
        panic!("Thread number exceeded");
    }

    /// Add a new thread
    /// Calls action with tid and thread context
    pub fn add(&self, mut context: Box<Context>) -> Tid {
        let (tid, mut thread) = self.alloc_tid();
        context.set_tid(tid);
        *thread = Some(Thread {
            status: Status::Ready,
            status_after_stop: Status::Ready,
            waiter: None,
            context: Some(context),
        });
        self.scheduler.push(tid);
        tid
    }

    /// Make thread `tid` time slice -= 1.
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

    /// Set the priority of thread `tid`
    pub fn set_priority(&self, tid: Tid, priority: u8) {
        self.scheduler.set_priority(tid, priority);
    }

    /// Called by Processor to get a thread to run.
    /// The manager first mark it `Running`,
    /// then take out and return its Context.
    pub(crate) fn run(&self, cpu_id: usize) -> Option<(Tid, Box<Context>)> {
        self.scheduler.pop(cpu_id).map(|tid| {
            let mut proc_lock = self.threads[tid].lock();
            let mut proc = proc_lock.as_mut().expect("thread not exist");
            proc.status = Status::Running(cpu_id);
            (tid, proc.context.take().expect("context not exist"))
        })
    }

    /// Called by Processor to finish running a thread
    /// and give its context back.
    pub(crate) fn stop(&self, tid: Tid, context: Box<Context>) {
        let mut proc_lock = self.threads[tid].lock();
        let proc = proc_lock.as_mut().expect("thread not exist");
        proc.status = proc.status_after_stop.clone();
        proc.status_after_stop = Status::Ready;
        proc.context = Some(context);
        match proc.status {
            Status::Ready => self.scheduler.push(tid),
            Status::Exited(_) => self.exit_handler(proc),
            _ => {}
        }
    }

    /// Called by `JoinHandle` to let thread `tid` wait for `target`.
    /// The `tid` is going to sleep, and will be woke up when `target` exit.
    /// (see `exit_handler()`)
    pub(crate) fn wait(&self, tid: Tid, target: Tid) {
        self.set_status(tid, Status::Sleeping);
        let mut target_lock = self.threads[target].lock();
        let target = target_lock.as_mut().expect("thread not exist");
        target.waiter = Some(tid);
    }

    /// Switch the status of a thread.
    /// Insert/Remove it to/from scheduler if necessary.
    fn set_status(&self, tid: Tid, status: Status) {
        let mut proc_lock = self.threads[tid].lock();
        if let Some(mut proc) = proc_lock.as_mut() {
            trace!("thread {} {:?} -> {:?}", tid, proc.status, status);
            match (&proc.status, &status) {
                (Status::Ready, Status::Ready) => return,
                (Status::Ready, _) => panic!("can not remove a thread from ready queue"),
                (Status::Exited(_), _) => panic!("can not set status for a exited thread"),
                (Status::Sleeping, Status::Exited(_)) => self.timer.lock().stop(Event::Wakeup(tid)),
                (Status::Running(_), Status::Ready) => {} // thread will be added to scheduler in stop()
                (_, Status::Ready) => self.scheduler.push(tid),
                _ => {}
            }
            match proc.status {
                Status::Running(_) => proc.status_after_stop = status,
                _ => proc.status = status,
            }
            match proc.status {
                Status::Exited(_) => self.exit_handler(proc),
                _ => {}
            }
        }
    }

    /// Try to remove an exited thread `tid`.
    /// Return its exit code if success.
    pub fn try_remove(&self, tid: Tid) -> Option<ExitCode> {
        let mut proc_lock = self.threads[tid].lock();
        let proc = proc_lock.as_ref().expect("thread not exist");
        match proc.status {
            Status::Exited(code) => {
                // release the tid
                *proc_lock = None;
                Some(code)
            }
            _ => None,
        }
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
        let mut proc_lock = self.threads[tid].lock();
        if let Some(mut proc) = proc_lock.as_mut() {
            trace!("thread {} {:?} -> {:?}", tid, proc.status, Status::Ready);
            if let Status::Sleeping = proc.status {
                proc.status = Status::Ready;
                self.scheduler.push(tid);
            }
        }
    }

    pub fn exit(&self, tid: Tid, code: ExitCode) {
        // NOTE: if `tid` is running, status change will be deferred.
        self.set_status(tid, Status::Exited(code));
    }
    /// Called when a thread exit
    fn exit_handler(&self, proc: &mut Thread) {
        // wake up waiter
        if let Some(waiter) = proc.waiter {
            self.wakeup(waiter);
        }
        // drop its context
        proc.context = None;
    }
}

fn new_vec_default<T: Default>(size: usize) -> Vec<T> {
    let mut vec = Vec::new();
    vec.resize_with(size, Default::default);
    vec
}
