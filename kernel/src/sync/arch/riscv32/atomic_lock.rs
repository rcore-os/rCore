//! RISCV atomic is not currently supported by Rust.
//! This is a ugly workaround.

use core::cell::UnsafeCell;

extern {
    fn __atomic_load_4(src: *const u32) -> u32;
    fn __atomic_store_4(dst: *mut u32, val: u32);
    fn __atomic_compare_exchange_4(dst: *mut u32, expected: *mut u32, desired: u32) -> bool;
}

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
