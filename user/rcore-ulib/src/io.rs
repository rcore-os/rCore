use alloc::string::String;
use core::fmt::{self, Write};
use core::option::Option;

use crate::syscall::{sys_putc, sys_read, sys_write};

pub const STDIN: usize = 0;
pub const STDOUT: usize = 1;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::io::print(format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

pub fn print(args: fmt::Arguments) {
    StdOut.write_fmt(args).unwrap();
}

pub fn print_putc(args: fmt::Arguments) {
    SysPutc.write_fmt(args).unwrap();
}

pub fn getc() -> Option<u8> {
    let mut c = 0u8;
    let ret = sys_read(STDIN, &mut c, 1);
    match ret {
        1 => Some(c),
        0 => None,
        _ => panic!(),
    }
}

pub fn get_line() -> String {
    let mut s = String::new();
    loop {
        let ret = getc();
        match ret {
            None => return s,
            Some(byte) => {
                let c = byte as char;
                match c {
                    '\x08' | '\x7f' /* '\b' */ => {
                        if s.pop().is_some() {
                            print!("\x08 \x08");
                        }
                    }
                    ' '...'\x7e' => {
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
    }
}

pub fn putc(c: u8) {
    sys_putc(c);
}

struct StdOut;

struct SysPutc;

impl fmt::Write for StdOut {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if sys_write(STDOUT, s.as_ptr(), s.len()) >= 0 {
            Ok(())
        } else {
            Err(fmt::Error::default())
        }
    }
}

impl fmt::Write for SysPutc {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.bytes() {
            sys_putc(c);
        }
        Ok(())
    }
}

/* VFS flags */
// TODO: use bitflags
// flags for open: choose one of these
pub const O_RDONLY: usize = 0; // open for reading only
pub const O_WRONLY: usize = 1; // open for writing only
pub const O_RDWR: usize = 2; // open for reading and writing
// then or in any of these:
pub const O_CREAT: usize = 0x00000004; // create file if it does not exist
pub const O_EXCL: usize = 0x00000008; // error if O_CREAT and the file exists
pub const O_TRUNC: usize = 0x00000010; // truncate file upon open
pub const O_APPEND: usize = 0x00000020; // append on each write