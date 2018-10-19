use core::sync::atomic::{AtomicBool, Ordering};

pub struct AtomicLock
{
    lock: AtomicBool
}

impl AtomicLock 
{
    pub fn new() -> AtomicLock {
        AtomicLock {
            lock: AtomicBool::new(false)
        }
    }

    pub fn try_lock(&self) -> bool {
        self.lock.compare_and_swap(false, true, Ordering::Acquire) == false
    }

    pub fn load(&self) -> bool {
        self.lock.load(Ordering::Relaxed)
    }

    pub fn store(&self) {
        self.lock.store(false, Ordering::Release);
    }
}

pub const ATOMIC_LOCK_INIT: AtomicLock = AtomicLock {
    lock: AtomicBool::new(false)
};