
#![no_std]
#![no_main]
#![feature(alloc)]

extern crate alloc;
#[macro_use]
extern crate rcore_user;



use alloc::vec::Vec;
use core::ptr;

use rcore_user::io::get_line;


use rcore_user::syscall::{sys_sleep, sys_vfork, sys_wait, sys_get_time, sys_exit};


pub fn sleep(time: usize) -> i32 {
    sys_sleep(time)
}

pub fn gettime_msec() -> u32{
    sys_get_time() as u32
}

pub fn fork() -> i32 {
    sys_vfork()
}

pub fn waitpid(pid: usize, code: *mut i32) -> i32 {
    sys_wait(pid, code)
}

pub fn exit(error_code: usize) {
    sys_exit(error_code);
    println!("BUG: exit failed.");
    while true {};
}



fn sleepy(pid: usize) {
    let time: usize = 1;
    for i in 0..10 {
        sleep(time);
        println!("sleep {} x {} slices.", i + 1, time);
    }
    exit(0);
}


// IMPORTANT: Must define main() like this
#[no_mangle]
pub fn main(){
    let time = gettime_msec();
    let mut pid1: usize = 0;
    let mut exit_code = 0;

    pid1 = fork() as usize;

    if pid1 == 0 {
        sleepy(pid1);
    } else {
        println!("child id is {}", pid1);
    }

    assert_eq!(waitpid(pid1, &mut exit_code), pid1 as i32);
    assert_eq!(exit_code, 0);

    println!("use {} msecs: {} to {}.", gettime_msec() - time, time, gettime_msec());
    println!("sleep pass.");
}
