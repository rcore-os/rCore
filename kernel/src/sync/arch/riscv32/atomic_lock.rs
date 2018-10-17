//! RISCV atomic is not currently supported by Rust.
//! This is a ugly workaround.


use arch::compiler_rt::{__atomic_compare_exchange_4, __atomic_store_4, __atomic_load_4};
use core::cell::UnsafeCell;

pub struct AtomicLock
{
    lock: UnsafeCell<u32>
}

impl AtomicLock 
{
    pub fn new() -> Self {
        AtomicLock {
            lock: UnsafeCell::new(0)
        }
    }

    /// Returns 1 if lock is acquired
    pub fn try_lock(&self) -> bool {
        let mut expected: u32 = 0;
        unsafe {
            __atomic_compare_exchange_4(self.lock.get(), &mut expected as *mut u32, 1)
        }
    }

    pub fn load(&self) -> bool {
        unsafe {
            __atomic_load_4(self.lock.get()) == 1
        }
    }

    pub fn store(&self) {
        unsafe {
            __atomic_store_4(self.lock.get(), 0);
        }
    }
}
