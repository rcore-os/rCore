//! Workaround for missing compiler-builtin symbols
//!
//! [atomic](http://llvm.org/docs/Atomics.html#libcalls-atomic)

#[link(name = "atomic_rt")]
extern {
    fn __atomic_load_1_workaround(src: *const u8) -> u8;
    fn __atomic_load_2_workaround(src: *const u16) -> u16;
    fn __atomic_load_4_workaround(src: *const u32) -> u32;
    fn __atomic_store_1_workaround(dst: *mut u8, val: u8);
    fn __atomic_store_4_workaround(dst: *mut u32, val: u32);
    fn __atomic_compare_exchange_1_workaround(dst: *mut u8, expected: *mut u8, desired: u8) -> bool;
    fn __atomic_compare_exchange_4_workaround(dst: *mut u32, expected: *mut u32, desired: u32) -> bool;
}

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
    __atomic_load_1_workaround(src)
}

#[no_mangle]
pub unsafe extern fn __atomic_load_2(src: *const u16) -> u16 {
    __atomic_load_2_workaround(src)
}

#[no_mangle]
pub unsafe extern fn __atomic_load_4(src: *const u32) -> u32 {
    __atomic_load_4_workaround(src)
}

#[no_mangle]
pub unsafe extern fn __atomic_store_1(dst: *mut u8, val: u8) {
    __atomic_store_1_workaround(dst, val);
}

#[no_mangle]
pub unsafe extern fn __atomic_store_4(dst: *mut u32, val: u32) {
    __atomic_store_4_workaround(dst, val);
}

// unsafe fn __atomic_compare_exchange<T: PartialEq>(dst: *mut T, expected: *mut T, desired: T) -> bool {
//     // use super::interrupt;
//     // let flags = interrupt::disable_and_store();
//     // let val = read(dst);
//     // let success = val == read(expected);
//     // write(dst, if success {desired} else {val});
//     // interrupt::restore(flags);
//     // success
//     // let mut val: T;
//     // asm!("lr.w $0, ($1)" : "=r"(val) : "r"(dst) : "memory" : "volatile");
//     // if val == *expected {
//     //     let mut sc_ret = 0;
//     //     asm!("sc.w $0, $1, ($2)" : "=r"(sc_ret) : "r"(desired), "r"(dst) : "memory" : "volatile");
//     //     return sc_ret == 0
//     // }
//     false
// }

#[no_mangle]
pub unsafe extern fn __atomic_compare_exchange_1(dst: *mut u8, expected: *mut u8, desired: u8) -> bool {
    __atomic_compare_exchange_1_workaround(dst, expected, desired)
}

#[no_mangle]
pub unsafe extern fn __atomic_compare_exchange_4(dst: *mut u32, expected: *mut u32, desired: u32) -> bool {
    __atomic_compare_exchange_4_workaround(dst, expected, desired)
}