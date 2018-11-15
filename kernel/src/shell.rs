//! Kernel shell

use alloc::string::String;
use alloc::vec::Vec;
use fs::ROOT_INODE;
use process::*;


pub fn shell() {
    let files = ROOT_INODE.list().unwrap();
    println!("Available programs: {:?}", files);

    const BUF_SIZE: usize = 0x40000;
    let mut buf = Vec::with_capacity(BUF_SIZE);
    unsafe { buf.set_len(BUF_SIZE); }
    loop {
        print!(">> ");
        let cmd = get_line();
        if cmd == "" {
            continue;
        }
        let name = cmd.split(' ').next().unwrap();
        if let Ok(file) = ROOT_INODE.lookup(name) {
            let len = file.read_at(0, &mut *buf).unwrap();
            let pid = processor().manager().add(ContextImpl::new_user(&buf[..len], cmd.split(' ')));
            processor().manager().wait(thread::current().id(), pid);
            processor().yield_now();
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
    ::fs::STDIN.pop()
}
