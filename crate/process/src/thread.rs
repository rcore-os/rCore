//! Thread std-like interface
//!
//! Based on Processor.
//! Used in the kernel.
//!
//! # Example
//!
//! ```
//! // Define a support implementation struct
//! pub struct ThreadSupportImpl;
//!
//! // Impl `ThreadSupport` trait
//! impl ThreadSupport for ThreadSupportImpl { ... }
//!
//! // Export the full struct as `thread`.
//! #[allow(non_camel_case_types)]
//! pub type thread = ThreadMod<ThreadSupportImpl>;
//! ```
//!
//! ```
//! // Use it just like `std::thread`
//! use thread;
//! let t = thread::current();
//!
//! // But the other struct is not available ...
//! let t: thread::Thread;   // ERROR!
//! ```

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use core::any::Any;
use core::marker::PhantomData;
use core::ptr;
use core::time::Duration;
use core::ops::DerefMut;
use processor::*;
use scheduler::Scheduler;

/// All dependencies for thread mod.
pub trait ThreadSupport {
    type Context: Context;
    type Scheduler: Scheduler;
    type ProcessorGuard: DerefMut<Target=Processor_<Self::Context, Self::Scheduler>>;
    fn processor() -> Self::ProcessorGuard;
}

/// Root structure served as thread mod
pub struct ThreadMod<S: ThreadSupport> {
    mark: PhantomData<S>
}

impl<S: ThreadSupport> ThreadMod<S> {
    /*
    **  @brief  Gets a handle to the thread that invokes it.
    **  @param  none
    **  @retval the thread to get
    */
    pub fn current() -> Thread<S> {
        Thread {
            pid: S::processor().current_pid(),
            mark: PhantomData,
        }
    }

    /*
    **  @brief  Puts the current thread to sleep for the specified amount of time.
    **  @param  dur: Duration           the time to sleep
    **  @retval none
    */
    pub fn sleep(dur: Duration) {
        let time = dur_to_ticks(dur);
        info!("sleep: {:?} ticks", time);
        let mut processor = S::processor();
        let pid = processor.current_pid();
        processor.sleep(pid, time);
        processor.schedule();

        fn dur_to_ticks(dur: Duration) -> usize {
            return dur.as_secs() as usize * 100 + dur.subsec_nanos() as usize / 10_000_000;
        }
    }

    /*
    **  @brief  Spawns a new thread, returning a JoinHandle for it.
    **  @param  f: F                    the thread to start
    **  @retval JoinHandle              the JoinHandle of the new thread
    */
    pub fn spawn<F, T>(f: F) -> JoinHandle<S, T>
        where
            F: Send + 'static + FnOnce() -> T,
            T: Send + 'static,
    {
        info!("spawn:");
        let f = Box::into_raw(Box::new(f));
        let pid = S::processor().add(Context::new_kernel(kernel_thread_entry::<S, F, T>, f as usize));
        return JoinHandle {
            thread: Thread { pid, mark: PhantomData },
            mark: PhantomData,
        };

        extern fn kernel_thread_entry<S, F, T>(f: usize) -> !
            where
                S: ThreadSupport,
                F: Send + 'static + FnOnce() -> T,
                T: Send + 'static,
        {
            let f = unsafe { Box::from_raw(f as *mut F) };
            let ret = Box::new(f());
//            unsafe { LocalKey::<usize>::get_map() }.clear();
            let mut processor = S::processor();
            let pid = processor.current_pid();
            processor.exit(pid, Box::into_raw(ret) as usize);
            processor.schedule();
            unreachable!()
        }
    }

    /*
    **  @brief  Cooperatively gives up a timeslice to the OS scheduler.
    **  @param  none
    **  @retval none
    */
    pub fn yield_now() {
        info!("yield:");
        let mut processor = S::processor();
        processor.set_reschedule();
        processor.schedule();
    }

    /*
    **  @brief  Blocks unless or until the current thread's token is made available.
    **  @param  none
    **  @retval none
    */
    pub fn park() {
        info!("park:");
        let mut processor = S::processor();
        let pid = processor.current_pid();
        processor.sleep_(pid);
        processor.schedule();
    }
}

/// A handle to a thread.
pub struct Thread<S: ThreadSupport> {
    pid: usize,
    mark: PhantomData<S>,
}

impl<S: ThreadSupport> Thread<S> {
    /*
    **  @brief  Atomically makes the handle's token available if it is not already.
    **  @param  none
    **  @retval none
    */
    pub fn unpark(&self) {
        let mut processor = S::processor();
        processor.wakeup_(self.pid);
    }
    /*
    **  @brief  Gets the thread's unique identifier.
    **  @param  none
    **  @retval usize           the the thread's unique identifier
    */
    pub fn id(&self) -> usize {
        self.pid
    }
}

/// An owned permission to join on a thread (block on its termination).
pub struct JoinHandle<S: ThreadSupport, T> {
    thread: Thread<S>,
    mark: PhantomData<T>,
}

impl<S: ThreadSupport, T> JoinHandle<S, T> {
    /*
    **  @brief  Extracts a handle to the underlying thread.
    **  @param  none
    **  @retval the thread of the handle
    */
    pub fn thread(&self) -> &Thread<S> {
        &self.thread
    }
    /*
    **  @brief  Waits for the associated thread to finish.
    **  @param  none
    **  @retval Result<T, ()>       the result of the associated thread
    */
    pub fn join(self) -> Result<T, ()> {
        let mut processor = S::processor();
        match processor.current_wait_for(self.thread.pid) {
            WaitResult::Ok(_, exit_code) => unsafe {
                Ok(*Box::from_raw(exit_code as *mut T))
            }
            WaitResult::NotExist => Err(()),
        }
    }
}

//pub struct LocalKey<T: 'static> {
//    init: fn() -> T,
//}
//
//impl<T: 'static> LocalKey<T> {
//    pub fn with<F, R>(&'static self, f: F) -> R
//        where F: FnOnce(&T) -> R
//    {
//        let map = unsafe { Self::get_map() };
//        let key = self as *const _ as usize;
//        if !map.contains_key(&key) {
//            map.insert(key, Box::new((self.init)()));
//        }
//        let value = map.get(&key).unwrap().downcast_ref::<T>().expect("type error");
//        f(value)
//    }
//    pub const fn new(init: fn() -> T) -> Self {
//        LocalKey { init }
//    }
//    /// Get `BTreeMap<usize, Box<Any>>` at the current kernel stack bottom
//    /// The stack must be aligned with 0x8000
//    unsafe fn get_map() -> &'static mut BTreeMap<usize, Box<Any>> {
//        const STACK_SIZE: usize = 0x8000;
//        let stack_var = 0usize;
//        let ptr = (&stack_var as *const _ as usize) / STACK_SIZE * STACK_SIZE;
//        let map = unsafe { &mut *(ptr as *mut Option<BTreeMap<usize, Box<Any>>>) };
//        if map.is_none() {
//            *map = Some(BTreeMap::new());
//        }
//        map.as_mut().unwrap()
//    }
//}
//
//pub mod test {
//    use thread;
//    use core::cell::RefCell;
//    use core::time::Duration;
//
//    pub fn unpack() {
//        let parked_thread = thread::spawn(|| {
//            println!("Parking thread");
//            thread::park();
//            println!("Thread unparked");
//            5
//        });
//
//        // Let some time pass for the thread to be spawned.
//        thread::sleep(Duration::from_secs(2));
//
//        println!("Unpark the thread");
//        parked_thread.thread().unpark();
//
//        let ret = parked_thread.join().unwrap();
//        assert_eq!(ret, 5);
//    }
//
//    pub fn local_key() {
//        static FOO: thread::LocalKey<RefCell<usize>> = thread::LocalKey::new(|| RefCell::new(1));
//
//        FOO.with(|f| {
//            assert_eq!(*f.borrow(), 1);
//            *f.borrow_mut() = 2;
//        });
//
//        // each thread starts out with the initial value of 1
//        thread::spawn(move || {
//            FOO.with(|f| {
//                assert_eq!(*f.borrow(), 1);
//                *f.borrow_mut() = 3;
//            });
//        }).join();
//
//        // we retain our original value of 2 despite the child thread
//        FOO.with(|f| {
//            assert_eq!(*f.borrow(), 2);
//        });
//        println!("local key success");
//    }
//}
