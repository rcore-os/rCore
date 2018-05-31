//! Thread std-like interface
//!
//! Based on process mod.
//! Used in the kernel.

use process::*;
use core::marker::PhantomData;
use core::ptr;
use alloc::boxed::Box;

/// Gets a handle to the thread that invokes it.
pub fn current() -> Thread {
    Thread {
        pid: PROCESSOR.try().unwrap().lock().current_pid(),
    }
}

/// Puts the current thread to sleep for the specified amount of time.
pub fn sleep(time: usize) {
    // TODO: use core::time::Duration
    info!("sleep: {} ticks", time);
    let mut processor = PROCESSOR.try().unwrap().lock();
    let pid = processor.current_pid();
    processor.sleep(pid, time);
    processor.schedule();
}

/// Spawns a new thread, returning a JoinHandle for it.
pub fn spawn<F, T>(f: F) -> JoinHandle<T>
    where
        F: Send + 'static + FnOnce() -> T,
        T: Send + 'static,
{
    use process;
    let pid = process::add_kernel_process(kernel_thread_entry::<F, T>, &f as *const _ as usize);
    return JoinHandle {
        thread: Thread { pid },
        mark: PhantomData,
    };

    extern fn kernel_thread_entry<F, T>(f: usize) -> !
        where
            F: Send + 'static + FnOnce() -> T,
            T: Send + 'static,
    {
        debug!("kernel_thread_entry");
        let f = unsafe { ptr::read(f as *mut F) };
        let ret = Box::new(f());
        let mut processor = PROCESSOR.try().unwrap().lock();
        let pid = processor.current_pid();
        processor.exit(pid, Box::into_raw(ret) as usize);
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

pub mod test {
    use thread;

    pub fn unpack() {
        let parked_thread = thread::spawn(|| {
            println!("Parking thread");
            thread::park();
            println!("Thread unparked");
            5
        });

        // Let some time pass for the thread to be spawned.
        thread::sleep(200);

        println!("Unpark the thread");
        parked_thread.thread().unpark();

        let ret = parked_thread.join().unwrap();
        assert_eq!(ret, 5);
    }
}
