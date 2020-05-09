//! A counting, blocking, semaphore.
//!
//! Same as [std::sync::Semaphore at rust 1.7.0](https://docs.rs/std-semaphore/0.1.0/std_semaphore/)

use super::Condvar;
use super::SpinNoIrqLock as Mutex;
use crate::syscall::SysError;
use core::cell::Cell;
use core::ops::Deref;

struct SemaphoreInner {
    pub count: isize,
    pub pid: usize,
    pub removed: bool,
}

/// A counting, blocking, semaphore.
pub struct Semaphore {
    // value and removed
    lock: Mutex<SemaphoreInner>,
    cvar: Condvar,
}

/// An RAII guard which will release a resource acquired from a semaphore when
/// dropped.
pub struct SemaphoreGuard<'a> {
    sem: &'a Semaphore,
}

impl Semaphore {
    /// Creates a new semaphore with the initial count specified.
    ///
    /// The count specified can be thought of as a number of resources, and a
    /// call to `acquire` or `access` will block until at least one resource is
    /// available. It is valid to initialize a semaphore with a negative count.
    pub fn new(count: isize) -> Semaphore {
        Semaphore {
            lock: Mutex::new(SemaphoreInner {
                count,
                removed: false,
                pid: 0,
            }),
            cvar: Condvar::new(),
        }
    }

    pub fn remove(&self) {
        self.lock.lock().removed = false;
        self.cvar.notify_all();
    }

    /// Acquires a resource of this semaphore, blocking the current thread until
    /// it can do so.
    ///
    /// This method will block until the internal count of the semaphore is at
    /// least 1.
    pub fn acquire(&self) -> Result<(), SysError> {
        let mut inner = self.lock.lock();
        while !inner.removed && inner.count <= 0 {
            inner = self.cvar.wait(inner);
        }
        if inner.removed {
            Err(SysError::EIDRM)
        } else {
            inner.count -= 1;
            Ok(())
        }
    }

    /// Release a resource from this semaphore.
    ///
    /// This will increment the number of resources in this semaphore by 1 and
    /// will notify any pending waiters in `acquire` or `access` if necessary.
    pub fn release(&self) {
        self.lock.lock().count += 1;
        self.cvar.notify_one();
    }

    /// Acquires a resource of this semaphore, returning an RAII guard to
    /// release the semaphore when dropped.
    ///
    /// This function is semantically equivalent to an `acquire` followed by a
    /// `release` when the guard returned is dropped.
    pub fn access(&self) -> Result<SemaphoreGuard, SysError> {
        self.acquire()?;
        Ok(SemaphoreGuard { sem: self })
    }

    /// Get the current count
    pub fn get(&self) -> isize {
        self.lock.lock().count
    }
    pub fn get_ncnt(&self) -> usize {
        self.cvar.wait_queue_len()
    }

    pub fn get_pid(&self) -> usize {
        self.lock.lock().pid
    }
    pub fn set_pid(&self, pid: usize) {
        self.lock.lock().pid = pid;
    }

    /// Set the current count
    pub fn set(&self, value: isize) {
        self.lock.lock().count = value;
    }

    /// Modify by k atomically. when wait is false avoid waiting. unused
    pub fn modify(&self, k: isize, wait: bool) -> Result<usize, ()> {
        if k > 0 {
            self.lock.lock().count += k;
            self.cvar.notify_one();
        } else if k <= 0 {
            let mut inner = self.lock.lock();
            let mut temp_k = k;
            while inner.count + temp_k < 0 {
                if wait == false {
                    return Err(());
                }
                temp_k += inner.count;
                inner.count = 0;
                inner = self.cvar.wait(inner);
            }
            inner.count += temp_k;
            if inner.count > 0 {
                self.cvar.notify_one();
            }
        }
        Ok(0)
    }
}

impl<'a> Drop for SemaphoreGuard<'a> {
    fn drop(&mut self) {
        self.sem.release();
    }
}

impl<'a> Deref for SemaphoreGuard<'a> {
    type Target = Semaphore;

    fn deref(&self) -> &Self::Target {
        return self.sem;
    }
}
