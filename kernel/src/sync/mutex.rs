//! Mutex (Spin, SpinNoIrq, Thread)
//!
//! Modified from spin::mutex.
//!
//! 一个可替换底层支持的锁框架。
//!
//! # 在此框架下实现了以下几种锁
//!
//! * `SpinLock`: 自旋锁。
//!     等价于`spin::Mutex`，相当于Linux中的`spin_lock`。
//!     当获取锁失败时，忙等待。
//!     由于没有禁用内核抢占和中断，在单处理器上使用可能发生死锁。
//!
//! * `SpinNoIrqLock`: 禁止中断的自旋锁。
//!     相当于Linux中的`spin_lock_irqsave`。
//!     在尝试获取锁之前禁用中断，在try_lock失败/解锁时恢复之前的中断状态。
//!     可被用于中断处理中，不会发生死锁。
//!
//! * `ThreadLock`: 线程调度锁。
//!     等价于`std::sync::Mutex`，依赖于`thread`模块提供线程调度支持。
//!     在获取锁失败时，将自己加入等待队列，让出CPU；在解锁时，唤醒一个等待队列中的线程。
//!
//! # 实现方法
//!
//! 由一个struct提供底层支持，它impl trait `MutexSupport`，并嵌入`Mutex`中。
//! `MutexSupport`提供了若干接口，它们会在操作锁的不同时间点被调用。
//! 注意这个接口实际是取了几种实现的并集，并不是很通用。

use super::Condvar;
use crate::arch::interrupt;
use crate::processor;
use core::cell::UnsafeCell;
use core::fmt;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

pub type SpinLock<T> = Mutex<T, Spin>;
pub type SpinNoIrqLock<T> = Mutex<T, SpinNoIrq>;
pub type SleepLock<T> = Mutex<T, Condvar>;

pub struct Mutex<T: ?Sized, S: MutexSupport> {
    lock: AtomicBool,
    support: S,
    user: UnsafeCell<(usize, usize)>, // (cid, tid)
    data: UnsafeCell<T>,
}

/// A guard to which the protected data can be accessed
///
/// When the guard falls out of scope it will release the lock.
pub struct MutexGuard<'a, T: ?Sized + 'a, S: MutexSupport + 'a> {
    pub(super) mutex: &'a Mutex<T, S>,
    support_guard: S::GuardData,
}

// Same unsafe impls as `std::sync::Mutex`
unsafe impl<T: ?Sized + Send, S: MutexSupport> Sync for Mutex<T, S> {}

unsafe impl<T: ?Sized + Send, S: MutexSupport> Send for Mutex<T, S> {}

impl<T, S: MutexSupport> Mutex<T, S> {
    /// Creates a new spinlock wrapping the supplied data.
    ///
    /// May be used statically:
    ///
    /// ```
    /// #![feature(const_fn)]
    /// use spin;
    ///
    /// static MUTEX: spin::Mutex<()> = spin::Mutex::new(());
    ///
    /// fn demo() {
    ///     let lock = MUTEX.lock();
    ///     // do something with lock
    ///     drop(lock);
    /// }
    /// ```
    pub fn new(user_data: T) -> Mutex<T, S> {
        Mutex {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(user_data),
            support: S::new(),
            user: UnsafeCell::new((0, 0)),
        }
    }

    /// Consumes this mutex, returning the underlying data.
    pub fn into_inner(self) -> T {
        // We know statically that there are no outstanding references to
        // `self` so there's no need to lock.
        let Mutex { data, .. } = self;
        data.into_inner()
    }
}

impl<T: ?Sized, S: MutexSupport> Mutex<T, S> {
    fn obtain_lock(&self) {
        while self.lock.compare_and_swap(false, true, Ordering::Acquire) != false {
            let mut try_count = 0;
            // Wait until the lock looks unlocked before retrying
            while self.lock.load(Ordering::Relaxed) {
                self.support.cpu_relax();
                try_count += 1;
                if try_count == 0x100000 {
                    let (cid, tid) = unsafe { *self.user.get() };
                    error!(
                        "Mutex: deadlock detected! locked by cpu {} thread {} @ {:?}",
                        cid, tid, self as *const Self
                    );
                }
            }
        }
        let cid = crate::arch::cpu::id();
        let tid = processor().tid_option().unwrap_or(0);
        unsafe { self.user.get().write((cid, tid)) };
    }

    /// Locks the spinlock and returns a guard.
    ///
    /// The returned value may be dereferenced for data access
    /// and the lock will be dropped when the guard falls out of scope.
    ///
    /// ```
    /// let mylock = spin::Mutex::new(0);
    /// {
    ///     let mut data = mylock.lock();
    ///     // The lock is now locked and the data can be accessed
    ///     *data += 1;
    ///     // The lock is implicitly dropped
    /// }
    ///
    /// ```
    pub fn lock(&self) -> MutexGuard<T, S> {
        let support_guard = S::before_lock();
        self.obtain_lock();
        MutexGuard {
            mutex: self,
            support_guard,
        }
    }

    /// Force unlock the spinlock.
    ///
    /// This is *extremely* unsafe if the lock is not held by the current
    /// thread. However, this can be useful in some instances for exposing the
    /// lock to FFI that doesn't know how to deal with RAII.
    ///
    /// If the lock isn't held, this is a no-op.
    pub unsafe fn force_unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }

    /// Tries to lock the mutex. If it is already locked, it will return None. Otherwise it returns
    /// a guard within Some.
    pub fn try_lock(&self) -> Option<MutexGuard<T, S>> {
        let support_guard = S::before_lock();
        if self.lock.compare_and_swap(false, true, Ordering::Acquire) == false {
            Some(MutexGuard {
                mutex: self,
                support_guard,
            })
        } else {
            None
        }
    }
}

impl<T: ?Sized + fmt::Debug, S: MutexSupport + fmt::Debug> fmt::Debug for Mutex<T, S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_lock() {
            Some(guard) => write!(
                f,
                "Mutex {{ data: {:?}, support: {:?} }}",
                &*guard, self.support
            ),
            None => write!(f, "Mutex {{ <locked>, support: {:?} }}", self.support),
        }
    }
}

impl<T: ?Sized + Default, S: MutexSupport> Default for Mutex<T, S> {
    fn default() -> Mutex<T, S> {
        Mutex::new(Default::default())
    }
}

impl<'a, T: ?Sized, S: MutexSupport> Deref for MutexGuard<'a, T, S> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T: ?Sized, S: MutexSupport> DerefMut for MutexGuard<'a, T, S> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<'a, T: ?Sized, S: MutexSupport> Drop for MutexGuard<'a, T, S> {
    /// The dropping of the MutexGuard will release the lock it was created from.
    fn drop(&mut self) {
        self.mutex.lock.store(false, Ordering::Release);
        self.mutex.support.after_unlock();
    }
}

/// Low-level support for mutex
pub trait MutexSupport {
    type GuardData;
    fn new() -> Self;
    /// Called when failing to acquire the lock
    fn cpu_relax(&self);
    /// Called before lock() & try_lock()
    fn before_lock() -> Self::GuardData;
    /// Called when MutexGuard dropping
    fn after_unlock(&self);
}

/// Spin lock
#[derive(Debug)]
pub struct Spin;

impl MutexSupport for Spin {
    type GuardData = ();

    fn new() -> Self {
        Spin
    }
    fn cpu_relax(&self) {
        unsafe {
            #[cfg(target_arch = "x86_64")]
            asm!("pause" :::: "volatile");
            #[cfg(any(target_arch = "riscv32", target_arch = "riscv64", target_arch = "mips"))]
            asm!("nop" :::: "volatile");
            #[cfg(target_arch = "aarch64")]
            asm!("yield" :::: "volatile");
        }
    }
    fn before_lock() -> Self::GuardData {}
    fn after_unlock(&self) {}
}

/// Spin & no-interrupt lock
#[derive(Debug)]
pub struct SpinNoIrq;

/// Contains RFLAGS before disable interrupt, will auto restore it when dropping
pub struct FlagsGuard(usize);

impl Drop for FlagsGuard {
    fn drop(&mut self) {
        unsafe { interrupt::restore(self.0) };
    }
}

impl FlagsGuard {
    pub fn no_irq_region() -> Self {
        Self(unsafe { interrupt::disable_and_store() })
    }
}

impl MutexSupport for SpinNoIrq {
    type GuardData = FlagsGuard;
    fn new() -> Self {
        SpinNoIrq
    }
    fn cpu_relax(&self) {
        unsafe {
            #[cfg(target_arch = "x86_64")]
            asm!("pause" :::: "volatile");
            #[cfg(any(target_arch = "riscv32", target_arch = "riscv64", target_arch = "mips"))]
            asm!("nop" :::: "volatile");
            #[cfg(target_arch = "aarch64")]
            asm!("yield" :::: "volatile");
        }
    }
    fn before_lock() -> Self::GuardData {
        FlagsGuard(unsafe { interrupt::disable_and_store() })
    }
    fn after_unlock(&self) {}
}

impl MutexSupport for Condvar {
    type GuardData = ();
    fn new() -> Self {
        Condvar::new()
    }
    fn cpu_relax(&self) {
        self._wait();
    }
    fn before_lock() -> Self::GuardData {}
    fn after_unlock(&self) {
        self.notify_one();
    }
}
