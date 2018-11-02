use core::ops::Deref;
use alloc::string::String;
use alloc::collections::VecDeque;
use sync::Condvar;
use sync::SpinNoIrqLock as Mutex;

pub fn get_line() -> String {
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

#[derive(Default)]
pub struct InputQueue {
    buf: Mutex<VecDeque<char>>,
    pushed: Condvar,
}

impl InputQueue {
    pub fn push(&self, c: char) {
        self.buf.lock().push_back(c);
        self.pushed.notify_one();
    }
    pub fn pop(&self) -> char {
        loop {
            let ret = self.buf.lock().pop_front();
            match ret {
                Some(c) => return c,
                None => self.pushed._wait(),
            }
        }
    }
}

lazy_static! {
    pub static ref CONSOLE_INPUT: InputQueue = InputQueue::default();
}

pub fn get_char() -> char {
    CONSOLE_INPUT.pop()
}
