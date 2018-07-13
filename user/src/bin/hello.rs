#![no_std]
#![no_main]

#[macro_use]
extern crate ucore_ulib;

// IMPORTANT: Must define main() like this
#[no_mangle]
pub fn main() {
    println!("Hello uCore!");
}
