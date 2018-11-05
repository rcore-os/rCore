use alloc::boxed::Box;
use alloc::sync::Arc;
use spin::Mutex;
use core::cell::UnsafeCell;
use process_manager::*;

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

    pub fn context(&self) -> &Context {
        &*self.inner().proc.as_ref().unwrap().1
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
