use crate::trap::NAIVE_TIMER;
use crate::{
    arch::timer::timer_now,
    sync::SpinNoIrqLock as Mutex,
    syscall::{SysError, SysResult},
};
use alloc::boxed::Box;
use alloc::{collections::VecDeque, sync::Arc};
use core::pin::Pin;
use core::task::{Context, Poll};
use core::{future::Future, task::Waker, time::Duration};

pub struct Waiter {
    waker: Option<Waker>,
    woken: bool,
    futex: Arc<Futex>,
}

pub struct FutexInner {
    waiters: VecDeque<Arc<Mutex<Waiter>>>,
}

pub struct Futex {
    pub inner: Mutex<FutexInner>,
}

impl Futex {
    pub fn new() -> Self {
        Futex {
            inner: Mutex::new(FutexInner {
                waiters: VecDeque::new(),
            }),
        }
    }

    pub fn wake(&self, wake_count: usize) -> usize {
        let mut inner = self.inner.lock();
        for i in 0..wake_count {
            if let Some(waiter) = inner.waiters.pop_front() {
                let mut waiter = waiter.lock();
                waiter.woken = true;
                if let Some(waker) = waiter.waker.take() {
                    waker.wake();
                }
            } else {
                return i;
            }
        }
        wake_count
    }

    pub fn wait(self: &Arc<Self>, timeout: Option<Duration>) -> impl Future<Output = SysResult> {
        #[must_use = "future does nothing unless polled/`await`-ed"]
        struct FutexFuture {
            waiter: Arc<Mutex<Waiter>>,
            deadline: Option<Duration>,
        }

        impl Future for FutexFuture {
            type Output = SysResult;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut inner = self.waiter.lock();
                // check wakeup
                if inner.woken {
                    return Poll::Ready(Ok(0));
                }
                if let Some(deadline) = self.deadline {
                    if timer_now() >= deadline {
                        inner.woken = true;
                        return Poll::Ready(Err(SysError::ETIMEDOUT));
                    }
                }

                // first time?
                if inner.waker.is_none() {
                    // futex
                    let mut futex = inner.futex.inner.lock();
                    futex.waiters.push_back(self.waiter.clone());
                    drop(futex);
                    inner.waker.replace(cx.waker().clone());

                    // timer
                    if let Some(deadline) = self.deadline {
                        let waker = cx.waker().clone();
                        NAIVE_TIMER
                            .lock()
                            .add(deadline, Box::new(move |_| waker.wake()));
                    }
                }
                Poll::Pending
            }
        }

        FutexFuture {
            waiter: Arc::new(Mutex::new(Waiter {
                waker: None,
                woken: false,
                futex: self.clone(),
            })),
            deadline: timeout.map(|t| timer_now() + t),
        }
    }
}
