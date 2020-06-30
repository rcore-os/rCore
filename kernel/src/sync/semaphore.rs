//! A counting, blocking, semaphore.
//!
//! Same as [std::sync::Semaphore at rust 1.7.0](https://docs.rs/std-semaphore/0.1.0/std_semaphore/)

use super::{Event, EventBus, SpinNoIrqLock as Mutex};
use crate::syscall::SysError;
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::cell::Cell;
use core::future::Future;
use core::ops::Deref;
use core::pin::Pin;
use core::task::{Context, Poll};

/// A counting, blocking, semaphore.
pub struct Semaphore {
    // value and removed
    lock: Arc<Mutex<SemaphoreInner>>,
}

struct SemaphoreInner {
    count: isize,
    pid: usize,
    removed: bool,
    eventbus: EventBus,
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
            lock: Arc::new(Mutex::new(SemaphoreInner {
                count,
                removed: false,
                pid: 0,
                eventbus: EventBus::default(),
            })),
        }
    }

    pub fn remove(&self) {
        let mut inner = self.lock.lock();
        inner.removed = true;
        inner.eventbus.set(Event::SEMAPHORE_REMOVED);
    }

    /// Acquires a resource of this semaphore, blocking the current thread until
    /// it can do so.
    ///
    /// This method will block until the internal count of the semaphore is at
    /// least 1.
    pub async fn acquire(&self) -> Result<(), SysError> {
        #[must_use = "future does nothing unless polled/`await`-ed"]
        struct SemaphoreFuture {
            inner: Arc<Mutex<SemaphoreInner>>,
        }

        impl<'a> Future for SemaphoreFuture {
            type Output = Result<(), SysError>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
                let mut inner = self.inner.lock();
                if inner.removed {
                    return Poll::Ready(Err(SysError::EIDRM));
                } else if inner.count >= 1 {
                    inner.count -= 1;
                    if inner.count < 1 {
                        inner.eventbus.clear(Event::SEMAPHORE_CAN_ACQUIRE);
                    }
                    return Poll::Ready(Ok(()));
                }

                let waker = cx.waker().clone();
                inner.eventbus.subscribe(Box::new({
                    move |_| {
                        waker.wake_by_ref();
                        true
                    }
                }));

                return Poll::Pending;
            }
        }

        let future = SemaphoreFuture {
            inner: self.lock.clone(),
        };
        future.await
    }

    /// Release a resource from this semaphore.
    ///
    /// This will increment the number of resources in this semaphore by 1 and
    /// will notify any pending waiters in `acquire` or `access` if necessary.
    pub fn release(&self) {
        let mut inner = self.lock.lock();
        inner.count += 1;
        if inner.count >= 1 {
            inner.eventbus.set(Event::SEMAPHORE_CAN_ACQUIRE);
        }
    }

    /// Acquires a resource of this semaphore, returning an RAII guard to
    /// release the semaphore when dropped.
    ///
    /// This function is semantically equivalent to an `acquire` followed by a
    /// `release` when the guard returned is dropped.
    pub async fn access(&self) -> Result<SemaphoreGuard<'_>, SysError> {
        self.acquire().await?;
        Ok(SemaphoreGuard { sem: self })
    }

    /// Get the current count
    pub fn get(&self) -> isize {
        self.lock.lock().count
    }

    pub fn get_ncnt(&self) -> usize {
        self.lock.lock().eventbus.get_callback_len()
    }

    pub fn get_pid(&self) -> usize {
        self.lock.lock().pid
    }

    pub fn set_pid(&self, pid: usize) {
        self.lock.lock().pid = pid;
    }

    /// Set the current count
    pub fn set(&self, value: isize) {
        let mut inner = self.lock.lock();
        inner.count = value;
        if inner.count >= 1 {
            inner.eventbus.set(Event::SEMAPHORE_CAN_ACQUIRE);
        }
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
