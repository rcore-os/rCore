//! Thread std-like interface
//!
//! Based on process mod.
//! Used in the kernel.

use alloc::boxed::Box;
use alloc::BTreeMap;
use core::any::Any;
use core::marker::PhantomData;
use core::ptr;
use core::time::Duration;
use process::*;

/// Gets a handle to the thread that invokes it.
pub fn current() -> Thread {
    Thread {
        pid: PROCESSOR.try().unwrap().lock().current_pid(),
    }
}

/// Puts the current thread to sleep for the specified amount of time.
pub fn sleep(dur: Duration) {
    info!("sleep: {:?}", dur);
    let mut processor = PROCESSOR.try().unwrap().lock();
    let pid = processor.current_pid();
    processor.sleep(pid, dur_to_ticks(dur));
    processor.schedule();

    fn dur_to_ticks(dur: Duration) -> usize {
        return dur.as_secs() as usize * 100 + dur.subsec_nanos() as usize / 10_000_000;
    }
}

/// Spawns a new thread, returning a JoinHandle for it.
pub fn spawn<F, T>(f: F) -> JoinHandle<T>
    where
        F: Send + 'static + FnOnce() -> T,
        T: Send + 'static,
{
    info!("spawn:");
    use process;
    let f = Box::into_raw(Box::new(f));
    let pid = process::add_kernel_process(kernel_thread_entry::<F, T>, f as usize);
    return JoinHandle {
        thread: Thread { pid },
        mark: PhantomData,
    };

    extern fn kernel_thread_entry<F, T>(f: usize) -> !
        where
            F: Send + 'static + FnOnce() -> T,
            T: Send + 'static,
    {
        let f = unsafe { Box::from_raw(f as *mut F) };
        let ret = Box::new(f());
        let mut processor = PROCESSOR.try().unwrap().lock();
        let pid = processor.current_pid();
        processor.exit(pid, Box::into_raw(ret) as usize);
        processor.schedule();
        unreachable!()
    }
}

/// Cooperatively gives up a timeslice to the OS scheduler.
pub fn yield_now() {
    info!("yield:");
    let mut processor = PROCESSOR.try().unwrap().lock();
    processor.set_reschedule();
    processor.schedule();
}

/// Blocks unless or until the current thread's token is made available.
pub fn park() {
    info!("park:");
    let mut processor = PROCESSOR.try().unwrap().lock();
    let pid = processor.current_pid();
    processor.sleep_(pid);
    processor.schedule();
}

/// A handle to a thread.
pub struct Thread {
    pid: usize,
}

impl Thread {
    /// Atomically makes the handle's token available if it is not already.
    pub fn unpark(&self) {
        let mut processor = PROCESSOR.try().unwrap().lock();
        processor.wakeup_(self.pid);
    }
    /// Gets the thread's unique identifier.
    pub fn id(&self) -> usize {
        self.pid
    }
}

/// An owned permission to join on a thread (block on its termination).
pub struct JoinHandle<T> {
    thread: Thread,
    mark: PhantomData<T>,
}

impl<T> JoinHandle<T> {
    /// Extracts a handle to the underlying thread.
    pub fn thread(&self) -> &Thread {
        &self.thread
    }
    /// Waits for the associated thread to finish.
    pub fn join(self) -> Result<T, ()> {
        let mut processor = PROCESSOR.try().unwrap().lock();
        match processor.current_wait_for(self.thread.pid) {
            WaitResult::Ok(_, exit_code) => {
                unsafe {
                    let value = Box::from_raw(exit_code as *mut T);
                    Ok(ptr::read(exit_code as *const T))
                }
            }
            WaitResult::NotExist => Err(()),
        }
    }
}

pub struct LocalKey<T: 'static + Default> {
    init: fn() -> T,
}

impl<T: 'static + Default> LocalKey<T> {
    pub fn with<F, R>(&'static self, f: F) -> R
        where F: FnOnce(&T) -> R
    {
        let (map, key) = self.get_map_key();
        if !map.contains_key(&key) {
            map.insert(key, Box::new((self.init)()));
        }
        let value = map.get(&key).unwrap().downcast_ref::<T>().expect("type error");
        f(value)
    }
    pub const fn new() -> Self {
        LocalKey { init: Self::_init }
    }
    fn _init() -> T { T::default() }
    /// A `BTreeMap<usize, Box<Any>>` at kernel stack bottom
    /// The stack must be aligned with 0x8000
    fn get_map_key(&self) -> (&mut BTreeMap<usize, Box<Any>>, usize) {
        const STACK_SIZE: usize = 0x8000;
        let stack_var = 0usize;
        let ptr = (&stack_var as *const _ as usize) / STACK_SIZE * STACK_SIZE;
        let map = unsafe { &mut *(ptr as *mut Option<BTreeMap<usize, Box<Any>>>) };
        if map.is_none() {
            *map = Some(BTreeMap::new());
        }
        let map = map.as_mut().unwrap();
        let key = self as *const _ as usize;
        (map, key)
    }
}

pub mod test {
    use thread;
    use core::cell::RefCell;
    use core::time::Duration;

    pub fn unpack() {
        let parked_thread = thread::spawn(|| {
            println!("Parking thread");
            thread::park();
            println!("Thread unparked");
            5
        });

        // Let some time pass for the thread to be spawned.
        thread::sleep(Duration::from_secs(2));

        println!("Unpark the thread");
        parked_thread.thread().unpark();

        let ret = parked_thread.join().unwrap();
        assert_eq!(ret, 5);
    }

    pub fn local_key() {
        static FOO: thread::LocalKey<RefCell<usize>> = thread::LocalKey::new();

        FOO.with(|f| {
            assert_eq!(*f.borrow(), 0);
            *f.borrow_mut() = 2;
        });

        // each thread starts out with the initial value of 1
        thread::spawn(move || {
            FOO.with(|f| {
                assert_eq!(*f.borrow(), 0);
                *f.borrow_mut() = 3;
            });
        }).join();

        // we retain our original value of 2 despite the child thread
        FOO.with(|f| {
            assert_eq!(*f.borrow(), 2);
        });
        println!("local key success");
    }
}
