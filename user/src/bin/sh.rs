#![no_std]
#![no_main]

#[macro_use]
extern crate ucore_ulib;
use ucore_ulib::io::getc;
use ucore_ulib::syscall::{sys_exec, sys_fork, sys_wait};

pub fn get_line(buffer: &mut [u8]) -> usize {
    let mut pos: usize = 0;
    loop {
        let ret = getc();
        match ret {
            None => break,
            Some(byte) => {
                let c = byte as char;
                match c {
					'\u{8}' /* '\b' */ => {
						if pos > 0 {
                    		print!("\u{8} \u{8}");
							pos-=1;
						}
					}
					' '...'\u{7e}' => {
						if pos<buffer.len() {
							buffer[pos]=byte;
						}
						pos+=1;
						print!("{}", c);
					}
					'\n' | '\r' => {
						print!("\n");
						break;
					}
					_ => {}
				}
            }
        }
    }
    pos
}

const BUFFER_SIZE: usize = 4096;

// IMPORTANT: Must define main() like this
#[no_mangle]
pub fn main() -> i32 {
    use core::mem::uninitialized;
    let mut buf: [u8; BUFFER_SIZE] = unsafe { uninitialized() };
    println!("Rust user shell");
    loop {
        print!(">> ");
        let len = get_line(&mut buf);
        if len > BUFFER_SIZE {
            println!("Command is too long!");
        } else {
            let cmd = &buf[..len];
            let mut parsed: [u8; BUFFER_SIZE + 1] = unsafe { uninitialized() };
            let mut offset: [usize; BUFFER_SIZE + 1] = unsafe { uninitialized() };
            let mut start: usize = 0;
            let mut pos: usize = 0;
            let mut is_word = false;
            let mut parsed_pos: usize = 0;
            let mut offset_pos: usize = 0;
            loop {
                if pos >= cmd.len() {
                    if is_word {
                        offset[offset_pos] = parsed_pos;
                        offset_pos += 1;
                        parsed[parsed_pos..parsed_pos + pos - start]
                            .copy_from_slice(&cmd[start..pos]);
                        parsed_pos += pos - start;
                        parsed[parsed_pos] = 0;
                        // parsed_pos+=1;
                    }
                    break;
                }
                if cmd[pos] == (' ' as u8) {
                    if is_word {
                        is_word = false;
                        offset[offset_pos] = parsed_pos;
                        offset_pos += 1;
                        parsed[parsed_pos..parsed_pos + pos - start]
                            .copy_from_slice(&cmd[start..pos]);
                        parsed_pos += pos - start;
                        parsed[parsed_pos] = 0;
                        parsed_pos += 1;
                    }
                } else {
                    if !is_word {
                        is_word = true;
                        start = pos;
                    }
                }
                pos += 1;
            }
            if offset_pos > 0 {
                let pid = sys_fork();
                if pid == 0 {
                    let mut ptrs: [*const u8; BUFFER_SIZE] = unsafe { uninitialized() };
                    for i in 0..offset_pos {
                        ptrs[i] = unsafe { parsed.as_ptr().offset(offset[i] as isize) };
                    }
                    return sys_exec(parsed.as_ptr(), offset_pos, ptrs.as_ptr());
                } else if pid < 0 {
                    panic!("pid<0")
                } else {
                    let mut code: i32 = unsafe { uninitialized() };
                    sys_wait(pid as usize, &mut code as *mut i32);
                    println!("\n[Process exited with code {}]", code);
                }
            }
        }
    }
}
