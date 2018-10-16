//! Workaround for missing compiler-builtin symbols
//!
//! [atomic](http://llvm.org/docs/Atomics.html#libcalls-atomic)

/// Copy from:
/// https://github.com/rust-lang-nursery/compiler-builtins/blob/master/src/riscv32.rs
#[no_mangle]
pub extern fn __mulsi3(mut a: u32, mut b: u32) -> u32 {
    let mut r: u32 = 0;

    while a > 0 {
        if a & 1 > 0 {
            r += b;
        }
        a >>= 1;
        b <<= 1;
    }

    r
}

#[no_mangle]
pub extern fn abort() {
    loop {}
}

use core::ptr::{read, write};

#[no_mangle]
pub unsafe extern fn __atomic_load_1(src: *const u8) -> u8 {
    let mut res: u8 = 0;
    asm!("amoadd.w.rl $0, zero, ($1)" : "=r"(res) : "r"(src) : "memory" : "volatile");
    res
}

#[no_mangle]
pub unsafe extern fn __atomic_load_2(src: *const u16) -> u16 {
    let mut res: u16 = 0;
    asm!("amoadd.w.rl $0, zero, ($1)" : "=r"(res) : "r"(src) : "memory" : "volatile");
    res
}

#[no_mangle]
pub unsafe extern fn __atomic_load_4(src: *const u32) -> u32 {
    let mut res: u32 = 0;
    asm!("amoadd.w.rl $0, zero, ($1)" : "=r"(res) : "r"(src) : "memory" : "volatile");
    res
}

#[no_mangle]
pub unsafe extern fn __atomic_store_1(dst: *mut u8, val: u8) {
    asm!("amoswap.w.aq zero, $0, ($1)" :: "r"(val), "r"(dst) : "memory" : "volatile");
}

#[no_mangle]
pub unsafe extern fn __atomic_store_4(dst: *mut u32, val: u32) {
    asm!("amoswap.w.aq zero, $0, ($1)" :: "r"(val), "r"(dst) : "memory" : "volatile");
}

unsafe fn __atomic_compare_exchange<T: PartialEq>(dst: *mut T, expected: *mut T, desired: T) -> bool {
    // use super::interrupt;
    // let flags = interrupt::disable_and_store();
    // let val = read(dst);
    // let success = val == read(expected);
    // write(dst, if success {desired} else {val});
    // interrupt::restore(flags);
    // success
    let mut val: T;
    asm!("lr.w $0, ($1)" : "=r"(val) : "r"(dst) : "memory" : "volatile");
    if val == *expected {
        let mut sc_ret = 0;
        asm!("sc.w $0, $1, ($2)" : "=r"(sc_ret) : "r"(desired), "r"(dst) : "memory" : "volatile");
        return sc_ret == 0
    }
    false
}

#[no_mangle]
pub unsafe extern fn __atomic_compare_exchange_1(dst: *mut u8, expected: *mut u8, desired: u8) -> bool {
    __atomic_compare_exchange(dst, expected, desired)
}

#[no_mangle]
pub unsafe extern fn __atomic_compare_exchange_4(dst: *mut u32, expected: *mut u32, desired: u32) -> bool {
    __atomic_compare_exchange(dst, expected, desired)
}