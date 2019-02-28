//! Kernel shell

use alloc::string::String;
use alloc::vec::Vec;
use crate::fs::{ROOT_INODE, INodeExt};
use crate::process::*;
use crate::thread;

pub fn run_user_shell() {
    use crate::net::server;
    processor().manager().add(Thread::new_kernel(server, 0), 0);
    if let Ok(inode) = ROOT_INODE.lookup("sh") {
        println!("Going to user mode shell.");
        println!("Use 'ls' to list available programs.");
        let data = inode.read_as_vec().unwrap();
        processor().manager().add(Thread::new_user(data.as_slice(), "sh".split(' ')), 0);
    } else {
        processor().manager().add(Thread::new_kernel(shell, 0), 0);
    }
}

pub extern fn shell(_arg: usize) -> ! {
    let files = ROOT_INODE.list().unwrap();
    println!("Available programs: {:?}", files);
    let mut history = Vec::new();

    loop {
        print!(">> ");
        let cmd = get_line(&mut history);
        if cmd == "" {
            continue;
        }
        let name = cmd.split(' ').next().unwrap();
        if let Ok(file) = ROOT_INODE.lookup(name) {
            let data = file.read_as_vec().unwrap();
            let pid = processor().manager().add(Thread::new_user(data.as_slice(), cmd.split(' ')), thread::current().id());
            unsafe { thread::JoinHandle::<()>::_of(pid) }.join().unwrap();
        } else {
            println!("Program not exist");
        }
    }
}

const BEL: u8 = 0x07u8;
const BS: u8 = 0x08u8;
const LF: u8 = 0x0au8;
const CR: u8 = 0x0du8;
const ESC: u8 = 0x1bu8;
const DEL: u8 = 0x7fu8;

fn get_line(history: &mut Vec<Vec<u8>>) -> String {
    let mut cursor = 0;
    let mut line_vec = Vec::with_capacity(512);
    let mut history_index = history.len();
    loop {
        match get_char() {
            BS | DEL => {
                // Backspace
                if cursor > 0 {
                    cursor -= 1;
                    line_vec.remove(cursor);

                    put_char(BS);
                    for byte in &line_vec[cursor..] {
                        put_char(*byte);
                    }
                    put_char(b' ');
                    for _i in cursor..line_vec.len() {
                        put_char(ESC);
                        put_char(b'[');
                        put_char(b'D');
                    }
                    put_char(ESC);
                    put_char(b'[');
                    put_char(b'D');
                } else {
                    put_char(BEL);
                }
            }
            CR | LF => {
                // Return
                put_char(CR);
                put_char(LF);
                break;
            }
            ESC => {
                match get_char() {
                    b'[' => {
                        match get_char() {
                            b'D' => {
                                // Left arrow
                                if cursor > 0 {
                                    cursor -= 1;
                                    put_char(ESC);
                                    put_char(b'[');
                                    put_char(b'D');
                                } else {
                                    put_char(BEL);
                                }
                            }
                            b'C' => {
                                // Right arrow
                                if cursor < line_vec.len() {
                                    cursor += 1;
                                    put_char(ESC);
                                    put_char(b'[');
                                    put_char(b'C');
                                } else {
                                    put_char(BEL);
                                }
                            }
                            direction @ b'A' | direction @ b'B' => {
                                if direction == b'A' && history_index > 0 {
                                    // Up arrow
                                    history_index -= 1;
                                } else if direction == b'B' && history.len() > 0 // usize underflow
                                    && history_index < history.len() - 1
                                {
                                    // Down arrow
                                    history_index += 1;
                                } else {
                                    put_char(BEL);
                                    continue;
                                }

                                for _ in 0..line_vec.len() {
                                    put_char(ESC);
                                    put_char(b'[');
                                    put_char(b'D');
                                }
                                for _ in 0..line_vec.len() {
                                    put_char(b' ');
                                }
                                for _ in 0..line_vec.len() {
                                    put_char(ESC);
                                    put_char(b'[');
                                    put_char(b'D');
                                }
                                line_vec = history[history_index].clone();
                                cursor = line_vec.len();
                                for byte in &line_vec {
                                    put_char(*byte);
                                }
                            }
                            _ => {
                                put_char(BEL);
                            }
                        }
                    }
                    _ => {
                        put_char(BEL);
                    }
                }
            }
            byte if byte.is_ascii_graphic() || byte == b' ' => {
                line_vec.insert(cursor, byte);
                for byte in &line_vec[cursor..] {
                    put_char(*byte);
                }
                cursor += 1;
                for _i in cursor..line_vec.len() {
                    put_char(BS);
                }
            }
            _ => {
                // unrecognized characters
                put_char(BEL);
            }
        }
    }

    history.push(line_vec.clone());
    String::from_utf8(line_vec).unwrap_or_default()
}

fn get_char() -> u8 {
    crate::fs::STDIN.pop() as u8
}

fn put_char(ch: u8) {
    print!("{}", ch as char);
}
