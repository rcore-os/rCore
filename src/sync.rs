//! Mutex (Spin, SpinNoIrq, Thread)
//!
//! Modified from spin::mutex.

use core::sync::atomic::{AtomicBool, ATOMIC_BOOL_INIT, Ordering};
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::fmt;
use arch::interrupt;

pub type SpinLock<T> = Mutex<T, Spin>;
pub type SpinNoIrqLock<T> = Mutex<T, SpinNoIrq>;
pub type ThreadLock<T> = Mutex<T, Thread>;

pub struct Mutex<T: ?Sized, S: MutexSupport>
{
    lock: AtomicBool,
    support: S,
    data: UnsafeCell<T>,
}

/// A guard to which the protected data can be accessed
///
/// When the guard falls out of scope it will release the lock.
pub struct MutexGuard<'a, T: ?Sized + 'a, S: MutexSupport + 'a>
{
    lock: &'a AtomicBool,
    data: &'a mut T,
    support: &'a S,
    support_guard: S::GuardData,
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
    pub fn new(user_data: T) -> Mutex<T, S> {
        Mutex {
            lock: ATOMIC_BOOL_INIT,
            data: UnsafeCell::new(user_data),
            support: S::new(),
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
                self.support.cpu_relax();
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
        let support_guard = S::before_lock();
        self.obtain_lock();
        MutexGuard {
            lock: &self.lock,
            data: unsafe { &mut *self.data.get() },
            support: &self.support,
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
                lock: &self.lock,
                data: unsafe { &mut *self.data.get() },
                support: &self.support,
                support_guard,
            })
        } else {
            None
        }
    }
}

impl<T: ?Sized + fmt::Debug, S: MutexSupport + fmt::Debug> fmt::Debug for Mutex<T, S>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_lock() {
            Some(guard) => write!(f, "Mutex {{ data: {:?}, support: {:?} }}", &*guard, self.support),
            None => write!(f, "Mutex {{ <locked>, support: {:?} }}", self.support),
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

    fn new() -> Self { Spin }
    fn cpu_relax(&self) {
        unsafe { asm!("pause" :::: "volatile"); }
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

impl MutexSupport for SpinNoIrq {
    type GuardData = FlagsGuard;
    fn new() -> Self {
        SpinNoIrq
    }
    fn cpu_relax(&self) {
        unsafe { asm!("pause" :::: "volatile"); }
    }
    fn before_lock() -> Self::GuardData {
        FlagsGuard(unsafe { interrupt::disable_and_store() })
    }
    fn after_unlock(&self) {}
}

use thread;
use alloc::VecDeque;

/// With thread support
pub struct Thread {
    wait_queue: SpinLock<VecDeque<thread::Thread>>,
}

impl MutexSupport for Thread {
    type GuardData = ();
    fn new() -> Self {
        Thread { wait_queue: SpinLock::new(VecDeque::new()) }
    }
    fn cpu_relax(&self) {
        self.wait_queue.lock().push_back(thread::current());
        thread::park();
    }
    fn before_lock() -> Self::GuardData {}
    fn after_unlock(&self) {
        if let Some(t) = self.wait_queue.lock().pop_front() {
            t.unpark();
        }
    }
}


pub mod philosopher {
    use thread;
    use core::time::Duration;
    use alloc::{arc::Arc, Vec};
    use super::ThreadLock as Mutex;

    struct Philosopher {
        name: &'static str,
        left: usize,
        right: usize,
    }

    impl Philosopher {
        fn new(name: &'static str, left: usize, right: usize) -> Philosopher {
            Philosopher {
                name,
                left,
                right,
            }
        }

        fn eat(&self, table: &Table) {
            let _left = table.forks[self.left].lock();
            let _right = table.forks[self.right].lock();

            println!("{} is eating.", self.name);
            thread::sleep(Duration::from_secs(1));
        }

        fn think(&self) {
            println!("{} is thinking.", self.name);
            thread::sleep(Duration::from_secs(1));
        }
    }

    struct Table {
        forks: Vec<Mutex<()>>,
    }

    pub fn philosopher() {
        let table = Arc::new(Table {
            forks: vec![
                Mutex::new(()),
                Mutex::new(()),
                Mutex::new(()),
                Mutex::new(()),
                Mutex::new(()),
            ]
        });

        let philosophers = vec![
            Philosopher::new("1", 0, 1),
            Philosopher::new("2", 1, 2),
            Philosopher::new("3", 2, 3),
            Philosopher::new("4", 3, 4),
            Philosopher::new("5", 0, 4),
        ];

        let handles: Vec<_> = philosophers.into_iter().map(|p| {
            let table = table.clone();

            thread::spawn(move || {
                for i in 0..5 {
                    p.think();
                    p.eat(&table);
                    println!("{} iter {} end.", p.name, i);
                }
            })
        }).collect();

        for h in handles {
            h.join().unwrap();
        }
        println!("philosophers dining end");
    }
}