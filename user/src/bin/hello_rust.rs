#![no_std]
#![no_main]

#[macro_use]
extern crate ucore_ulib;

// IMPORTANT: Must define main() like this
#[no_mangle]
pub fn main() {
    println!("Hello Rust uCore!");
    println!("I am process {}.", ucore_ulib::syscall::sys_getpid());
    println!("hello pass.");
}
