use alloc::collections::VecDeque;
use super::*;
use crate::thread;
use alloc::sync::Arc;

#[derive(Default)]
pub struct Condvar {
    wait_queue: SpinNoIrqLock<VecDeque<Arc<thread::Thread>>>,
}

impl Condvar {
    pub fn new() -> Self {
        Condvar::default()
    }

    pub fn _wait(&self) {
        self.wait_queue.lock().push_back(Arc::new(thread::current()));
        thread::park();
    }

    pub fn wait_any(condvars: &[&Condvar]) {
        let token = Arc::new(thread::current());
        for condvar in condvars {
            condvar.wait_queue.lock().push_back(token.clone());
        }
        thread::park();
    }

    pub fn wait<'a, T, S>(&self, guard: MutexGuard<'a, T, S>) -> MutexGuard<'a, T, S>
        where S: MutexSupport
    {
        let mutex = guard.mutex;
        drop(guard);
        self._wait();
        mutex.lock()
    }

    pub fn notify_one(&self) {
        if let Some(t) = self.wait_queue.lock().pop_front() {
            t.unpark();
        }
    }
    pub fn notify_all(&self) {
        while let Some(t) = self.wait_queue.lock().pop_front() {
            t.unpark();
        }
    }
    pub fn _clear(&self) {
        self.wait_queue.lock().clear();
    }
}