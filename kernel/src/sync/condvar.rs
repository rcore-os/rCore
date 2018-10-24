use alloc::collections::VecDeque;
use super::*;
use thread;

#[derive(Default)]
pub struct Condvar {
    wait_queue: SpinNoIrqLock<VecDeque<thread::Thread>>,
}

impl Condvar {
    pub fn new() -> Self {
        Condvar::default()
    }
    pub fn _wait(&self) {
        self.wait_queue.lock().push_back(thread::current());
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
}