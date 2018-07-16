use core::ops::Deref;
use alloc::string::String;
use arch::io::getchar;

pub fn get_line() -> String {
    let mut s = String::new();
    loop {
        let c = getchar();
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