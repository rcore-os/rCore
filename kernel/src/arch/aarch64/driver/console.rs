//! Framebuffer console display driver for ARM64

use super::board::fb::{FramebufferInfo, FRAME_BUFFER};
use alloc::vec::Vec;
use core::fmt::{self, Write};
use lazy_static::lazy_static;
use spin::Mutex;

#[derive(Debug, Clone, Copy)]
struct Color(u8);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ConsoleChar {
    ascii_char: u8,
    color: Color,
}

impl Default for ConsoleChar {
    fn default() -> Self {
        ConsoleChar {
            ascii_char: b' ',
            color: Color(0),
        }
    }
}

const CHAR_WIDTH: u16 = 8;
const CHAR_HEIGHT: u16 = 16;

/// Character buffer
struct ConsoleBuffer {
    num_row: u16,
    num_col: u16,
    buf: Vec<Vec<ConsoleChar>>,
}

impl ConsoleBuffer {
    fn new(num_row: u16, num_col: u16) -> ConsoleBuffer {
        ConsoleBuffer {
            num_row,
            num_col,
            buf: vec![vec![ConsoleChar::default(); num_col as usize]; num_row as usize],
        }
    }

    /// Read one character at `(row, col)`.
    fn read(&self, row: u16, col: u16) -> ConsoleChar {
        self.buf[row as usize][col as usize]
    }

    /// Write one character at `(row, col)`.
    /// TODO: font & color
    fn write(&mut self, row: u16, col: u16, ch: ConsoleChar) {
        self.buf[row as usize][col as usize] = ch;

        let off_x = col * CHAR_WIDTH;
        let off_y = row * CHAR_HEIGHT;
        if let Some(fb) = FRAME_BUFFER.lock().as_mut() {
            for y in 0..CHAR_HEIGHT {
                for x in 0..CHAR_WIDTH {
                    let pixel = if ch.color.0 == 0 { 0 } else { !0 };
                    fb.write((off_x + x) as u32, (off_y + y) as u32, pixel);
                }
            }
        }
    }

    /// Delete one character at `(row, col)`.
    fn delete(&mut self, row: u16, col: u16) {
        self.write(row, col, ConsoleChar::default());
    }

    /// Insert one blank line at the bottom and remove the top line.
    /// TODO: improve performance
    fn new_line(&mut self) {
        for i in 1..self.num_row {
            for j in 0..self.num_col {
                self.write(i - 1, j, self.read(i, j));
            }
        }
        for j in 0..self.num_col {
            self.write(self.num_row - 1, j, ConsoleChar::default());
        }
    }

    /// Clear the entire buffer and screen.
    fn clear(&mut self) {
        for i in 0..self.num_row as usize {
            for j in 0..self.num_col as usize {
                self.buf[i][j] = ConsoleChar::default()
            }
        }
        if let Some(fb) = FRAME_BUFFER.lock().as_mut() {
            fb.clear();
        }
    }
}

/// Console structure
pub struct Console {
    /// current color
    color: Color,
    /// cursor row
    row: u16,
    /// cursor column
    col: u16,
    /// number of rows
    num_row: u16,
    /// number of columns
    num_col: u16,
    /// character buffer
    buf: ConsoleBuffer,
}

impl Console {
    fn new(fb: &FramebufferInfo) -> Console {
        let num_row = fb.yres as u16 / CHAR_HEIGHT;
        let num_col = fb.xres as u16 / CHAR_WIDTH;
        Console {
            color: Color(1),
            row: 0,
            col: 0,
            num_row,
            num_col,
            buf: ConsoleBuffer::new(num_row, num_col),
        }
    }

    fn new_line(&mut self) {
        self.col = 0;
        if self.row < self.num_row - 1 {
            self.row += 1;
        } else {
            self.buf.new_line();
        }
    }

    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\x7f' => {
                if self.col > 0 {
                    self.col -= 1;
                    self.buf.delete(self.row, self.col);
                } else if self.row > 0 {
                    self.row -= 1;
                    self.col = self.num_col - 1;
                    self.buf.delete(self.row, self.col);
                }
            }
            b'\n' => self.new_line(),
            b'\r' => self.col = 0,
            byte => {
                if self.col >= self.num_col {
                    self.new_line();
                }

                let ch = ConsoleChar {
                    ascii_char: byte,
                    color: self.color,
                };
                self.buf.write(self.row, self.col, ch);
                self.col += 1;
            }
        }
    }

    pub fn clear(&mut self) {
        self.color = Color(1);
        self.row = 0;
        self.col = 0;
        self.buf.clear();
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte)
        }
        Ok(())
    }
}

lazy_static! {
    pub static ref CONSOLE: Mutex<Option<Console>> = Mutex::new(None);
}

/// Initialize console driver
pub fn init() {
    if let Some(fb) = FRAME_BUFFER.lock().as_ref() {
        *CONSOLE.lock() = Some(Console::new(&fb.fb_info));
    }

    if let Some(console) = CONSOLE.lock().as_mut() {
        console.write_str("Hello Raspberry Pi!\n").unwrap();
    }
}
