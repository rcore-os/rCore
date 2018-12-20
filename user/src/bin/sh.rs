#![no_std]
#![no_main]

#[macro_use]
extern crate ucore_ulib;
use ucore_ulib::io::getc;

pub fn get_line(buffer: &mut[u8]) -> usize {
	let mut pos: usize=0;
	loop {
		let ret=getc();
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
			},
		}
	}
	pos
}

const BUFFER_SIZE:usize=4096;

// IMPORTANT: Must define main() like this
#[no_mangle]
pub fn main() {
	use core::mem::uninitialized;
	let mut buf:[u8;BUFFER_SIZE] = unsafe { uninitialized() };
	println!("Rust user shell");
	loop {
        print!(">> ");
        let len = get_line(&mut buf);
		if len>BUFFER_SIZE {
			println!("Command is too long!");
			continue;
		}
		let command=&buf[..len];
        use core::str;
		println!("{}", unsafe{ str::from_utf8_unchecked(command) })
	}
}
