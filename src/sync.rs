//! Mutex (Spin, Spin-NoInterrupt, Yield)
//!
//! Modified from spin::mutex.

use core::sync::atomic::{AtomicBool, ATOMIC_BOOL_INIT, Ordering};
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::fmt;
use core::marker::PhantomData;
use arch::interrupt;

pub type SpinLock<T> = Mutex<T, Spin>;
pub type SpinNoIrqLock<T> = Mutex<T, SpinNoIrq>;
pub type YieldLock<T> = Mutex<T, Yield>;

/// Spin & no-interrupt lock
pub struct Mutex<T: ?Sized, S: MutexSupport>
{
    lock: AtomicBool,
    support: PhantomData<S>,
    data: UnsafeCell<T>,
}

/// A guard to which the protected data can be accessed
///
/// When the guard falls out of scope it will release the lock.
pub struct MutexGuard<'a, T: ?Sized + 'a, S: MutexSupport>
{
    lock: &'a AtomicBool,
    data: &'a mut T,
    support: S,
}

// Same unsafe impls as `std::sync::Mutex`
unsafe impl<T: ?Sized + Send, S: MutexSupport> Sync for Mutex<T, S> {}

unsafe impl<T: ?Sized + Send, S: MutexSupport> Send for Mutex<T, S> {}

impl<T, S: MutexSupport> Mutex<T, S>
{
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
    pub const fn new(user_data: T) -> Mutex<T, S> {
        Mutex {
            lock: ATOMIC_BOOL_INIT,
            data: UnsafeCell::new(user_data),
            support: PhantomData,
        }
    }

    /// Consumes this mutex, returning the underlying data.
    pub fn into_inner(self) -> T {
        // We know statically that there are no outstanding references to
        // `self` so there's no need to lock.
        let Mutex { data, .. } = self;
        unsafe { data.into_inner() }
    }
}

impl<T: ?Sized, S: MutexSupport> Mutex<T, S>
{
    fn obtain_lock(&self) {
        while self.lock.compare_and_swap(false, true, Ordering::Acquire) != false {
            // Wait until the lock looks unlocked before retrying
            while self.lock.load(Ordering::Relaxed) {
                S::cpu_relax();
            }
        }
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
    pub fn lock(&self) -> MutexGuard<T, S>
    {
        let support = S::before_lock();
        self.obtain_lock();
        MutexGuard {
            lock: &self.lock,
            data: unsafe { &mut *self.data.get() },
            support,
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
        let support = S::before_lock();
        if self.lock.compare_and_swap(false, true, Ordering::Acquire) == false {
            Some(MutexGuard {
                lock: &self.lock,
                data: unsafe { &mut *self.data.get() },
                support,
            })
        } else {
            support.after_unlock();
            None
        }
    }
}

impl<T: ?Sized + fmt::Debug, S: MutexSupport> fmt::Debug for Mutex<T, S>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_lock() {
            Some(guard) => write!(f, "Mutex<{:?}> {{ data: {:?} }}", self.support, &*guard),
            None => write!(f, "Mutex<{:?}> {{ <locked> }}", self.support),
        }
    }
}

impl<T: ?Sized + Default, S: MutexSupport> Default for Mutex<T, S> {
    fn default() -> Mutex<T, S> {
        Mutex::new(Default::default())
    }
}

impl<'a, T: ?Sized, S: MutexSupport> Deref for MutexGuard<'a, T, S>
{
    type Target = T;
    fn deref<'b>(&'b self) -> &'b T { &*self.data }
}

impl<'a, T: ?Sized, S: MutexSupport> DerefMut for MutexGuard<'a, T, S>
{
    fn deref_mut<'b>(&'b mut self) -> &'b mut T { &mut *self.data }
}

impl<'a, T: ?Sized, S: MutexSupport> Drop for MutexGuard<'a, T, S>
{
    /// The dropping of the MutexGuard will release the lock it was created from.
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
        self.support.after_unlock();
    }
}

/// Low-level support for mutex
pub trait MutexSupport {
    /// Called when failing to acquire the lock
    fn cpu_relax();
    /// Called before lock() & try_lock()
    fn before_lock() -> Self;
    /// Called when MutexGuard dropping & try_lock() failed
    fn after_unlock(&self);
}

/// Spin lock
pub struct Spin;

impl MutexSupport for Spin {
    fn cpu_relax() {
        unsafe { asm!("pause" :::: "volatile"); }
    }
    fn before_lock() -> Self {
        Spin
    }
    fn after_unlock(&self) {}
}

/// Spin & no-interrupt lock
pub struct SpinNoIrq {
    flags: usize,
}

impl MutexSupport for SpinNoIrq {
    fn cpu_relax() {
        unsafe { asm!("pause" :::: "volatile"); }
    }
    fn before_lock() -> Self {
        SpinNoIrq {
            flags: unsafe { interrupt::disable_and_store() },
        }
    }
    fn after_unlock(&self) {
        unsafe { interrupt::restore(self.flags) };
    }
}

/// With thread support
pub struct Yield;

impl MutexSupport for Yield {
    fn cpu_relax() {
        use thread;
        thread::yield_now();
    }
    fn before_lock() -> Self {
        unimplemented!()
    }
    fn after_unlock(&self) {
        unimplemented!()
    }
}