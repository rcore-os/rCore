/*
 * Rust BareBones OS
 * - By John Hodge (Mutabah/thePowersGang) 
 *
 * arch/x86/debug.rs
 * - Debug output channel
 *
 * Writes debug to the standard PC serial port (0x3F8 .. 0x3FF)
 * 
 * == LICENCE ==
 * This code has been put into the public domain, there are no restrictions on
 * its use, and the author takes no liability.
 */

use core::fmt;
use spin::Mutex;
use x86_64::instructions::port::{inb, outb};

pub static SERIAL: Mutex<Serial> = Mutex::new(Serial{});

pub struct Serial;

impl Serial {
	/// Write a single byte to the output channel
	pub fn write(&mut self, byte: u8) {
		unsafe {
			self.wait();
			
			// Send the byte out the serial port
			outb(0x3F8, byte);
			
			// Also send to the bochs 0xe9 hack
			outb(0xE9, byte);
		}
	}
	/// Wait for the serial port's fifo to not be empty
	unsafe fn wait(&self) {
		while (inb(0x3F8+5) & 0x20) == 0 {}
	}
}

impl fmt::Write for Serial {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
        	self.write(byte)
        }
        Ok(())
    }
}