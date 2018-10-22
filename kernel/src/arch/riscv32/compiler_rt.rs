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
    read(src)
}

#[no_mangle]
pub unsafe extern fn __atomic_load_2(src: *const u16) -> u16 {
    read(src)
}

#[no_mangle]
pub unsafe extern fn __atomic_load_4(src: *const u32) -> u32 {
    read(src)
}

#[no_mangle]
pub unsafe extern fn __atomic_store_1(dst: *mut u8, val: u8) {
    write(dst, val)
}

#[no_mangle]
pub unsafe extern fn __atomic_store_4(dst: *mut u32, val: u32) {
    write(dst, val)
}

unsafe fn __atomic_compare_exchange<T: PartialEq>(dst: *mut T, expected: *mut T, desired: T) -> bool {
    use super::interrupt;
    let flags = interrupt::disable_and_store();
    let val = read(dst);
    let success = val == read(expected);
    write(dst, if success {desired} else {val});
    interrupt::restore(flags);
    success
}

#[no_mangle]
pub unsafe extern fn __atomic_compare_exchange_1(dst: *mut u8, expected: *mut u8, desired: u8) -> bool {
    __atomic_compare_exchange(dst, expected, desired)
}

#[no_mangle]
pub unsafe extern fn __atomic_compare_exchange_4(dst: *mut u32, expected: *mut u32, desired: u32) -> bool {
    __atomic_compare_exchange(dst, expected, desired)
}


#[no_mangle]
pub unsafe extern fn __atomic_fetch_add_4(dst: *mut u32, delta: u32) -> u32 {
    use super::interrupt;
    let flags = interrupt::disable_and_store();
    let val = read(dst);
    let new_val = val + delta;
    write(dst, new_val);
    interrupt::restore(flags);
    val
}

#[no_mangle]
pub unsafe extern fn __atomic_fetch_sub_4(dst: *mut u32, delta: u32) -> u32 {
    use super::interrupt;
    let flags = interrupt::disable_and_store();
    let val = read(dst);
    let new_val = val - delta;
    write(dst, new_val);
    interrupt::restore(flags);
    val
}