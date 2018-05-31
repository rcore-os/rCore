//! Spin & no-interrupt lock
//!
//! Modified from spin::mutex.
//! Search 'interrupt::' for difference.

use core::sync::atomic::{AtomicBool, ATOMIC_BOOL_INIT, Ordering};
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::fmt;
use arch::interrupt;

/// Spin & no-interrupt lock
pub struct Mutex<T: ?Sized>
{
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

/// A guard to which the protected data can be accessed
///
/// When the guard falls out of scope it will release the lock.
pub struct MutexGuard<'a, T: ?Sized + 'a>
{
    lock: &'a AtomicBool,
    data: &'a mut T,
    flags: usize,
}

// Same unsafe impls as `std::sync::Mutex`
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}

impl<T> Mutex<T>
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
    pub const fn new(user_data: T) -> Mutex<T> {
        Mutex {
            lock: ATOMIC_BOOL_INIT,
            data: UnsafeCell::new(user_data),
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

impl<T: ?Sized> Mutex<T>
{
    fn obtain_lock(&self) {
        while self.lock.compare_and_swap(false, true, Ordering::Acquire) != false {
            // Wait until the lock looks unlocked before retrying
            while self.lock.load(Ordering::Relaxed) {
                unsafe { asm!("pause" :::: "volatile"); }
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
    pub fn lock(&self) -> MutexGuard<T>
    {
        let flags = unsafe { interrupt::disable_and_store() };
        self.obtain_lock();
        MutexGuard {
            lock: &self.lock,
            data: unsafe { &mut *self.data.get() },
            flags,
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
    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        let flags = unsafe { interrupt::disable_and_store() };
        if self.lock.compare_and_swap(false, true, Ordering::Acquire) == false {
            Some(MutexGuard {
                lock: &self.lock,
                data: unsafe { &mut *self.data.get() },
                flags,
            })
        } else {
            unsafe { interrupt::restore(flags) };
            None
        }
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Mutex<T>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_lock() {
            Some(guard) => write!(f, "Mutex {{ data: {:?} }}", &*guard),
            None => write!(f, "Mutex {{ <locked> }}"),
        }
    }
}

impl<T: ?Sized + Default> Default for Mutex<T> {
    fn default() -> Mutex<T> {
        Mutex::new(Default::default())
    }
}

impl<'a, T: ?Sized> Deref for MutexGuard<'a, T>
{
    type Target = T;
    fn deref<'b>(&'b self) -> &'b T { &*self.data }
}

impl<'a, T: ?Sized> DerefMut for MutexGuard<'a, T>
{
    fn deref_mut<'b>(&'b mut self) -> &'b mut T { &mut *self.data }
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T>
{
    /// The dropping of the MutexGuard will release the lock it was created from.
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
        unsafe { interrupt::restore(self.flags) };
    }
}