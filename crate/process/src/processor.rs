use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::Mutex;
use scheduler::Scheduler;
use core::cell::UnsafeCell;

/// Process executor
///
/// Per-CPU struct. Defined at global.
/// Only accessed by associated CPU with interrupt disabled.
#[derive(Default)]
pub struct Processor {
    inner: UnsafeCell<Option<ProcessorInner>>,
}

unsafe impl Sync for Processor {}

struct ProcessorInner {
    id: usize,
    proc: Option<(Pid, Box<Context>)>,
    loop_context: Box<Context>,
    manager: Arc<ProcessManager>,
}

impl Processor {
    pub const fn new() -> Self {
        Processor { inner: UnsafeCell::new(None) }
    }

    pub unsafe fn init(&self, id: usize, context: Box<Context>, manager: Arc<ProcessManager>) {
        unsafe {
            *self.inner.get() = Some(ProcessorInner {
                id,
                proc: None,
                loop_context: context,
                manager,
            });
        }
    }

    fn inner(&self) -> &mut ProcessorInner {
        unsafe { &mut *self.inner.get() }.as_mut()
            .expect("Processor is not initialized")
    }

    /// Begin running processes after CPU setup.
    ///
    /// This function never returns. It loops, doing:
    /// - choose a process to run
    /// - switch to start running that process
    /// - eventually that process transfers control
    ///   via switch back to the scheduler.
    pub fn run(&self) -> ! {
        let inner = self.inner();
        loop {
            let proc = inner.manager.run(inner.id);
            trace!("CPU{} begin running process {}", inner.id, proc.0);
            inner.proc = Some(proc);
            unsafe {
                inner.loop_context.switch_to(&mut *inner.proc.as_mut().unwrap().1);
            }
            let (pid, context) = inner.proc.take().unwrap();
            trace!("CPU{} stop running process {}", inner.id, pid);
            inner.manager.stop(pid, context);
        }
    }

    /// Called by process running on this Processor.
    /// Yield and reschedule.
    pub fn yield_now(&self) {
        let inner = self.inner();
        unsafe {
            inner.proc.as_mut().unwrap().1.switch_to(&mut *inner.loop_context);
        }
    }

    pub fn pid(&self) -> Pid {
        self.inner().proc.as_ref().unwrap().0
    }

    pub fn manager(&self) -> &ProcessManager {
        &*self.inner().manager
    }

    pub fn tick(&self) {
        let need_reschedule = self.manager().tick(self.pid());
        if need_reschedule {
            self.yield_now();
        }
    }
}

struct Process {
    id: Pid,
    status: Status,
    status_after_stop: Status,
    context: Option<Box<Context>>,
}

type Pid = usize;
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