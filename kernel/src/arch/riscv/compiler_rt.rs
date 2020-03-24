//! Workaround for missing compiler-builtin symbols
//!
//! [atomic](http://llvm.org/docs/Atomics.html#libcalls-atomic)

#[no_mangle]
pub extern "C" fn abort() {
    panic!("abort");
}
