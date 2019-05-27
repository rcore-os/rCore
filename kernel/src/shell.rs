//! Kernel shell

use crate::fs::ROOT_INODE;
use crate::process::*;
use alloc::string::String;
use alloc::vec::Vec;

#[cfg(not(feature = "run_cmdline"))]
pub fn add_user_shell() {
    // the busybox of alpine linux can not transfer env vars into child process
    // Now we use busybox from
    // https://raw.githubusercontent.com/docker-library/busybox/82bc0333a9ae148fbb4246bcbff1487b3fc0c510/musl/busybox.tar.xz -O busybox.tar.xz
    // This one can transfer env vars!
    // Why???

    //    #[cfg(target_arch = "x86_64")]
    //        let init_shell="/bin/busybox"; // from alpine linux
    //
    //    #[cfg(not(target_arch = "x86_64"))]
    #[cfg(not(feature = "board_rocket_chip"))]
    let init_shell = "/busybox"; //from docker-library

    // fd is not available on rocket chip
    #[cfg(feature = "board_rocket_chip")]
    let init_shell = "/rust/sh";

    #[cfg(target_arch = "x86_64")]
    let init_envs =
        vec!["PATH=/usr/sbin:/usr/bin:/sbin:/bin:/usr/x86_64-alpine-linux-musl/bin".into()];

    #[cfg(not(target_arch = "x86_64"))]
    let init_envs = Vec::new();

    let init_args = vec!["busybox".into(), "ash".into()];

    if let Ok(inode) = ROOT_INODE.lookup(init_shell) {
        processor()
            .manager()
            .add(Thread::new_user(&inode, init_shell, init_args, init_envs));
    } else {
        processor().manager().add(Thread::new_kernel(shell, 0));
    }
}

#[cfg(feature = "run_cmdline")]
pub fn add_user_shell() {
    use crate::drivers::CMDLINE;
    let cmdline = CMDLINE.read();
    let inode = ROOT_INODE.lookup(&cmdline).unwrap();
    processor().manager().add(Thread::new_user(
        &inode,
        &cmdline,
        cmdline.split(' ').map(|s| s.into()).collect(),
        Vec::new(),
    ));
}

pub extern "C" fn shell(_arg: usize) -> ! {
    let files = ROOT_INODE.list().unwrap();
    println!("Available programs: {:?}", files);
    let mut history = Vec::new();

    loop {
        print!(">> ");
        let cmd = get_line(&mut history);
        if cmd == "" {
            continue;
        }
        let name = cmd.trim().split(' ').next().unwrap();
        if let Ok(inode) = ROOT_INODE.lookup(name) {
            let _tid = processor().manager().add(Thread::new_user(
                &inode,
                &cmd,
                cmd.split(' ').map(|s| s.into()).collect(),
                Vec::new(),
            ));
        // TODO: wait until process exits, or use user land shell completely
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
                    put_char(ESC);
                    put_char(b'[');
                    put_char(b'D');
                }
            }
            _ => {
                // unrecognized characters
                put_char(BEL);
            }
        }
    }

    if line_vec.len() > 0 {
        history.push(line_vec.clone());
    }
    String::from_utf8(line_vec).unwrap_or_default()
}

fn get_char() -> u8 {
    crate::fs::STDIN.pop() as u8
}

fn put_char(ch: u8) {
    print!("{}", ch as char);
}
