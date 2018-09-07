#![no_std] // don't link the Rust standard library
#![cfg_attr(not(test), no_main)] // disable all Rust-level entry points
#![cfg_attr(test, allow(dead_code, unused_macros, unused_imports))]

#[macro_use]
extern crate ucore;

#[cfg(not(test))]
#[cfg(target_arch = "x86_64")]
#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    ucore::rust_main();
}