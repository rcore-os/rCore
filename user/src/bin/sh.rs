#![no_std]
#![no_main]
#![feature(alloc)]

extern crate alloc;
#[macro_use]
extern crate rcore_user;

use alloc::vec::Vec;

use rcore_user::io::get_line;
use rcore_user::syscall::{sys_exec, sys_fork, sys_wait};

// IMPORTANT: Must define main() like this
#[no_mangle]
pub fn main() -> i32 {
    println!("Rust user shell");
    loop {
        print!(">> ");
        let cmd = get_line();
        // split cmd, make argc & argv
        let cmd = cmd.replace(' ', "\0") + "\0";
        let ptrs: Vec<*const u8> = cmd.split('\0')
            .filter(|s| !s.is_empty()).map(|s| s.as_ptr()).collect();
        if ptrs.is_empty() {
            continue;
        }

        let pid = sys_fork();
        assert!(pid >= 0);
        if pid == 0 {
            return sys_exec(ptrs[0], ptrs.len(), ptrs.as_ptr());
        } else {
            let mut code: i32 = 0;
            sys_wait(pid as usize, &mut code);
            println!("\n[Process exited with code {}]", code);
        }
    }
}
