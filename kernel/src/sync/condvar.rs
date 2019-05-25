use super::*;
use crate::process::processor;
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
    #[deprecated(note = "this may leads to lost wakeup problem. please use `wait` instead.")]
    pub fn _wait(&self) {
        // The condvar might be notified between adding to queue and thread parking.
        // So park current thread before wait queue lock is freed.
        // Avoid racing
        let lock = self.add_to_wait_queue();
        thread::park_action(move || {
            drop(lock);
        });
    }

    fn add_to_wait_queue(&self) -> MutexGuard<VecDeque<Arc<thread::Thread>>, SpinNoIrq> {
        let mut lock = self.wait_queue.lock();
        lock.push_back(Arc::new(thread::current()));
        return lock;
    }

    /// Wait for condvar until condition() returns Some
    pub fn wait_event<T>(condvar: &Condvar, mut condition: impl FnMut() -> Option<T>) -> T {
        Self::wait_events(&[condvar], condition)
    }

    /// Wait for condvars until condition() returns Some
    pub fn wait_events<T>(condvars: &[&Condvar], mut condition: impl FnMut() -> Option<T>) -> T {
        let thread = thread::current();
        let tid = thread.id();
        let token = Arc::new(thread);
        for condvar in condvars {
            let mut lock = condvar.wait_queue.lock();
            lock.push_back(token.clone());
        }
        let mut locks = Vec::with_capacity(condvars.len());
        loop {
            for condvar in condvars {
                let mut lock = condvar.wait_queue.lock();
                locks.push(lock);
            }
            processor().manager().sleep(tid, 0);
            locks.clear();

            if let Some(res) = condition() {
                let _ = FlagsGuard::no_irq_region();
                processor().manager().cancel_sleeping(tid);
                for condvar in condvars {
                    let mut lock = condvar.wait_queue.lock();
                    lock.retain(|t| !Arc::ptr_eq(t, &token));
                }
                return res;
            }
            processor().yield_now();
        }
    }

    /// Park current thread and wait for this condvar to be notified.
    pub fn wait<'a, T, S>(&self, guard: MutexGuard<'a, T, S>) -> MutexGuard<'a, T, S>
    where
        S: MutexSupport,
    {
        let mutex = guard.mutex;
        let token = Arc::new(thread::current());
        let mut lock = self.wait_queue.lock();
        lock.push_back(token.clone());

        thread::park_action(move || {
            drop(lock);
            drop(guard);
        });
        let ret = mutex.lock();
        let mut lock = self.wait_queue.lock();
        lock.retain(|t| !Arc::ptr_eq(&t, &token));
        ret
    }

    pub fn notify_one(&self) {
        if let Some(t) = self.wait_queue.lock().front() {
            t.unpark();
        }
    }
    pub fn notify_all(&self) {
        let queue = self.wait_queue.lock();
        for t in queue.iter() {
            t.unpark();
        }
    }
    /// Notify up to `n` waiters.
    /// Return the number of waiters that were woken up.
    pub fn notify_n(&self, n: usize) -> usize {
        let mut count = 0;
        let queue = self.wait_queue.lock();
        for t in queue.iter() {
            if count >= n {
                break;
            }
            t.unpark();
            count += 1;
        }

        count
    }
}
