use core::ops::Deref;

pub struct LineBuf {
    buf: [u8; BUF_SIZE],
    len: usize,
}

pub struct LineBufGuard<'a>(&'a str);

const BUF_SIZE: usize = 256;

impl LineBuf {
    pub const fn new() -> Self {
        LineBuf {
            buf: [0; BUF_SIZE],
            len: 0,
        }
    }

    /// Put a char received from serial. Return str if get a line.
    pub fn push_u8<'a>(&'a mut self, c: u8) -> Option<LineBufGuard<'a>> {
        use alloc::str;
        match c {
            b' '...128 if self.len != BUF_SIZE => {
                self.buf[self.len] = c;
                self.len += 1;
            }
            8 /* '\b' */ if self.len != 0 => {
                self.len -= 1;
            }
            b'\n' | b'\r' => {
                let s = str::from_utf8(&self.buf[..self.len]).unwrap();
                self.len = 0;
                return Some(LineBufGuard(s));
            }
            _ => {}
        }
        None
    }
}

impl<'a> Deref for LineBufGuard<'a> {
    type Target = str;
    fn deref(&self) -> &str {
        self.0
    }
}

use alloc::string::String;
use arch::io::getchar;

pub fn get_line() -> String {
    let mut buf = LineBuf::new();
    loop {
        let mut c = 0;
        while c == 0 {
            c = getchar() as u8;
        }
        print!("{}", c as char);
        if let Some(line) = buf.push_u8(c) {
            return String::from(&*line);
        }
    }
}