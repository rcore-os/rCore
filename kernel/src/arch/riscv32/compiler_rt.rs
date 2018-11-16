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
