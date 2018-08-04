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

unsafe fn __atomic_compare_exchange<T: PartialEq>(dst: *mut T, old: T, new: T) -> (T, bool) {
    use super::interrupt;
    let flags = interrupt::disable_and_store();
    let ret = read(dst);
    if ret == old {
        write(dst, new);
    }
    interrupt::restore(flags);
    (ret, true)
}

#[no_mangle]
pub unsafe extern fn __atomic_compare_exchange_1(dst: *mut u8, old: u8, src: u8) -> (u8, bool) {
    __atomic_compare_exchange(dst, old, src)
}

#[no_mangle]
pub unsafe extern fn __atomic_compare_exchange_4(dst: *mut u32, old: u32, src: u32) -> (u32, bool) {
    __atomic_compare_exchange(dst, old, src)
}