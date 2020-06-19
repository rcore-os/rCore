use crate::{sync::SpinNoIrqLock as Mutex, syscall::SysResult};
use alloc::{collections::VecDeque, sync::Arc};
use core::pin::Pin;
use core::task::{Context, Poll};
use core::{future::Future, task::Waker};

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

    pub fn wait(self: &Arc<Self>) -> impl Future<Output = SysResult> {
        struct FutexFuture {
            waiter: Arc<Mutex<Waiter>>,
        }
        impl Future for FutexFuture {
            type Output = SysResult;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut inner = self.waiter.lock();
                // check wakeup
                if inner.woken {
                    return Poll::Ready(Ok(0));
                }
                // first time?
                if inner.waker.is_none() {
                    let mut futex = inner.futex.inner.lock();
                    futex.waiters.push_back(self.waiter.clone());
                    drop(futex);
                    inner.waker.replace(cx.waker().clone());
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
        }
    }
}
