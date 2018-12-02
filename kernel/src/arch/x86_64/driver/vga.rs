use core::fmt;
use spin::Mutex;
use volatile::Volatile;
use lazy_static::lazy_static;
use x86_64::instructions::port::Port;
use crate::logging::Color;
use crate::consts::KERNEL_OFFSET;

#[derive(Debug, Clone, Copy)]
struct ColorCode(u8);

impl ColorCode {
    const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ScreenChar {
    ascii_char: u8,
    color_code: ColorCode,
}

impl ScreenChar {
    pub const fn new(ascii_char: u8, foreground_color: Color, background_color: Color) -> Self {
        ScreenChar {
            ascii_char,
            color_code: ColorCode::new(foreground_color, background_color)
        }
    }
}

pub const BUFFER_HEIGHT: usize = 25;
pub const BUFFER_WIDTH: usize = 80;

pub struct VgaBuffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

impl VgaBuffer {
    pub fn clear(&mut self) {
        let blank = ScreenChar::new(b' ', Color::LightGray, Color::Black);
        for row in 0 .. BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                self.chars[row][col].write(blank);
            }
        }
    }
    pub fn write(&mut self, row: usize, col: usize, screen_char: ScreenChar) {
        self.chars[row][col].write(screen_char);
    }
    pub fn read(&self, row: usize, col: usize) -> ScreenChar {
        self.chars[row][col].read()
    }
    pub fn set_cursor_at(&self, row: usize, col: usize) {
        assert!(row < BUFFER_HEIGHT && col < BUFFER_WIDTH);
        let pos = row * BUFFER_WIDTH + col;
        unsafe {
            // Reference: Rustboot project
            Port::new(0x3d4).write(15u16);
            Port::new(0x3d5).write(pos as u8);
            Port::new(0x3d4).write(14u16);
            Port::new(0x3d5).write((pos >> 8) as u8);
        }
    }
}

lazy_static! {
	pub static ref VGA_WRITER: Mutex<VgaWriter> = Mutex::new(
		VgaWriter::new(unsafe{ &mut *((KERNEL_OFFSET + 0xb8000) as *mut VgaBuffer) })
	);
}

pub struct VgaWriter {
    column_position: usize,
    color: Color,
    buffer: &'static mut VgaBuffer,
}

impl VgaWriter {
    fn new(buffer: &'static mut VgaBuffer) -> Self {
        buffer.clear();
        VgaWriter {
            column_position: 0,
            color: Color::LightGray,
            buffer,
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
                self.buffer.write(row - 1, col, screen_char);
            }
        }
        let blank = ScreenChar::new(b' ', self.color, Color::Black);
        for col in 0..BUFFER_WIDTH {
            self.buffer.write(BUFFER_HEIGHT - 1, col, blank);
        }
        self.column_position = 0;
        self.buffer.set_cursor_at(BUFFER_HEIGHT - 1, 0);
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