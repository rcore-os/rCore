#![no_std]
#![no_main]

#[macro_use]
extern crate rcore_user;

// IMPORTANT: Must define main() like this
#[no_mangle]
pub fn main() {
    println!("Hello Rust uCore!");
    println!("I am process {}.", rcore_user::syscall::sys_getpid());
    println!("hello pass.");
}
