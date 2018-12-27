//! Kernel shell

use alloc::string::String;
use alloc::vec::Vec;
use crate::fs::{ROOT_INODE, INodeExt};
use crate::process::*;

pub fn run_user_shell() {
//    if let Ok(inode) = ROOT_INODE.lookup("sh") {
//        println!("Going to user mode shell.");
//        println!("Use 'ls' to list available programs.");
//        let data = inode.read_as_vec().unwrap();
//        processor().manager().add(Process::new_user(data.as_slice(), "sh".split(' ')), 0);
//    } else {
        processor().manager().add(Process::new_kernel(shell, 0), 0);
//    }
}

pub extern fn shell(_arg: usize) -> ! {
    let files = ROOT_INODE.list().unwrap();
    println!("Available programs: {:?}", files);

    loop {
        print!(">> ");
        let cmd = get_line();
        if cmd == "" {
            continue;
        }
        if cfg!(all(target_arch = "aarch64", feature = "board_raspi3")) && cmd == "irq enable" {
            unsafe{crate::arch::interrupt::enable();}
            continue;
        }
        if cfg!(all(target_arch = "aarch64", feature = "board_raspi3")) && cmd == "irq disable" {
            unsafe{crate::arch::interrupt::disable();}
            continue;
        }
        if cfg!(all(target_arch = "aarch64", feature = "board_raspi3")) && cmd == "usbinit" {
            crate::arch::board::usb::init();
            continue;
        }
        if cfg!(all(target_arch = "aarch64", feature = "board_raspi3")) && cmd == "usbshow" {
            test_usb();
            continue;
        }
        let name = cmd.split(' ').next().unwrap();
        if let Ok(file) = ROOT_INODE.lookup(name) {
            let data = file.read_as_vec().unwrap();
            let pid = processor().manager().add(Process::new_user(data.as_slice(), cmd.split(' ')), thread::current().id());
            unsafe { thread::JoinHandle::<()>::_of(pid) }.join().unwrap();
        } else {
            println!("Program not exist");
        }
    }
}

fn get_line() -> String {
    let mut s = String::new();
    loop {
        let c = get_char();
        match c {
            '\u{7f}' /* '\b' */ => {
                if s.pop().is_some() {
                    print!("\u{7f}");
                }
            }
            ' '...'\u{7e}' => {
                s.push(c);
                print!("{}", c);
            }
            '\n' | '\r' => {
                print!("\n");
                return s;
            }
            _ => {}
        }
    }
}

fn get_char() -> char {
    crate::fs::STDIN.pop()
}

// Kernel tests
#[cfg(all(target_arch = "aarch64", feature = "board_raspi3"))]
fn test_usb() {
    use crate::arch::board::usb::*;
    let root_ptr = get_root_hub();
    unsafe {
        UsbShowTree(root_ptr, 1, '+');
    }
}
