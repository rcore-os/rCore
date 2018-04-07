use spin::Mutex;
use core::fmt;
use arch::driver::vga::*;

lazy_static! {
	pub static ref VGA_WRITER: Mutex<VgaWriter> = Mutex::new(
		// It is the only user of VGA_BUFFER. So it's safe.
		VgaWriter::new(unsafe{ &mut *VGA_BUFFER.as_ptr() })
	);
}

pub struct VgaWriter {
    column_position: usize,
    color: Color,
    buffer: &'static mut VgaBuffer
}

impl VgaWriter {
	fn new(buffer: &'static mut VgaBuffer) -> Self {
		buffer.clear();
		VgaWriter {
			column_position: 0,
			color: Color::LightGray,
			buffer: buffer,
		}
	}

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

				self.buffer.write(row, col, ScreenChar::new(byte, self.color, Color::Black));
                self.column_position += 1;
                self.buffer.set_cursor_at(row, col);
            }
        }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
				let screen_char = self.buffer.read(row, col);
				self.buffer.write(row-1, col, screen_char);
            }
        }
        let blank = ScreenChar::new(b' ', self.color, Color::Black);        
		for col in 0..BUFFER_WIDTH {
			self.buffer.write(BUFFER_HEIGHT-1, col, blank);
		}
        self.column_position = 0;
        self.buffer.set_cursor_at(BUFFER_HEIGHT-1, 0);
    }
}

impl fmt::Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
        	self.write_byte(byte)
        }
        Ok(())
    }
}
