use super::*;
use crate::thread;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;

#[derive(Default)]
pub struct Condvar {
    wait_queue: SpinNoIrqLock<VecDeque<Arc<thread::Thread>>>,
}

impl Condvar {
    pub fn new() -> Self {
        Condvar::default()
    }

    /// Park current thread and wait for this condvar to be notified.
    pub fn _wait(&self) {
        // The condvar might be notified between adding to queue and thread parking.
        // So park current thread before wait queue lock is freed.
        // Avoid racing
        let lock = self.add_to_wait_queue();
        thread::park_action(move || {
            drop(lock);
        });
    }

    pub fn wait_any(condvars: &[&Condvar]) {
        let token = Arc::new(thread::current());
        // Avoid racing in the same way as the function above
        let mut locks = Vec::new();
        locks.reserve(condvars.len());
        for condvar in condvars {
            let mut lock = condvar.wait_queue.lock();
            lock.push_back(token.clone());
            locks.push(lock);
        }
        thread::park_action(move || {
            drop(locks);
        });
    }

    pub fn add_to_wait_queue(&self) -> MutexGuard<VecDeque<Arc<thread::Thread>>, SpinNoIrq> {
        let mut lock = self.wait_queue.lock();
        lock.push_back(Arc::new(thread::current()));
        return lock;
    }

    pub fn wait<'a, T, S>(&self, guard: MutexGuard<'a, T, S>) -> MutexGuard<'a, T, S>
    where
        S: MutexSupport,
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
    /// Notify up to `n` waiters.
    /// Return the number of waiters that were woken up.
    pub fn notify_n(&self, n: usize) -> usize {
        let mut count = 0;
        while count < n {
            if let Some(t) = self.wait_queue.lock().pop_front() {
                t.unpark();
                count += 1;
            } else {
                break;
            }
        }
        count
    }
    pub fn _clear(&self) {
        self.wait_queue.lock().clear();
    }
}
