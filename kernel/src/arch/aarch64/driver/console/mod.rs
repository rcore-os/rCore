//! Framebuffer console display driver for ARM64

mod color;
mod fonts;

use self::color::{ConsoleColor, ConsoleColor::*, FramebufferColor};
use self::fonts::{Font, Font8x16};

use super::fb::{ColorDepth::*, FramebufferInfo, FRAME_BUFFER};
use alloc::vec::Vec;
use core::fmt;
use core::marker::PhantomData;
use lazy_static::lazy_static;
use log::*;
use spin::Mutex;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct ColorPair<C: FramebufferColor> {
    foreground: C,
    background: C,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConsoleChar {
    ascii_char: u8,
    color: ColorPair<ConsoleColor>,
}

impl Default for ConsoleChar {
    fn default() -> Self {
        ConsoleChar {
            ascii_char: b' ',
            color: ColorPair {
                foreground: Black,
                background: Black,
            },
        }
    }
}

/// Character buffer
struct ConsoleBuffer<F: Font> {
    num_row: usize,
    num_col: usize,
    buf: Vec<Vec<ConsoleChar>>,
    font: PhantomData<F>,
}

impl<F: Font> ConsoleBuffer<F> {
    fn new(num_row: usize, num_col: usize) -> ConsoleBuffer<F> {
        ConsoleBuffer {
            num_row,
            num_col,
            buf: vec![vec![ConsoleChar::default(); num_col]; num_row],
            font: PhantomData,
        }
    }

    /// Write one character at `(row, col)`.
    fn write(&mut self, row: usize, col: usize, ch: ConsoleChar) {
        self.buf[row][col] = ch;

        let off_x = col * F::WIDTH;
        let off_y = row * F::HEIGHT;
        if let Some(fb) = FRAME_BUFFER.lock().as_mut() {
            let (foreground, background) = match fb.color_depth {
                ColorDepth16 => (
                    ch.color.foreground.pack16() as u32,
                    ch.color.background.pack16() as u32,
                ),
                ColorDepth32 => (
                    ch.color.foreground.pack32(),
                    ch.color.background.pack32(),
                ),
            };
            for y in 0..F::HEIGHT {
                for x in 0..F::WIDTH {
                    let pixel = if F::get(ch.ascii_char, x, y) {
                        foreground
                    } else {
                        background
                    };
                    fb.write((off_x + x) as u32, (off_y + y) as u32, pixel);
                }
            }
        }
    }

    /// Delete one character at `(row, col)`.
    fn delete(&mut self, row: usize, col: usize) {
        self.write(row, col, ConsoleChar::default());
    }

    /// Insert one blank line at the bottom, and scroll up one line.
    /// XXX: read framebuffer is toooo slow, do not use `fb.copy()`.
    fn new_line(&mut self) {
        for i in 1..self.num_row {
            for j in 0..self.num_col {
                if self.buf[i - 1][j] != self.buf[i][j] {
                    self.write(i - 1, j, self.buf[i][j]);
                }
            }
        }
        for j in 0..self.num_col {
            self.buf[self.num_row - 1][j] = ConsoleChar::default();
        }

        if let Some(fb) = FRAME_BUFFER.lock().as_mut() {
            let rowbytes = F::HEIGHT * fb.fb_info.pitch as usize;
            fb.fill(rowbytes * (self.num_row - 1), rowbytes, 0);
        }
    }

    /// Clear the entire buffer and screen.
    fn clear(&mut self) {
        for i in 0..self.num_row {
            for j in 0..self.num_col {
                self.buf[i][j] = ConsoleChar::default()
            }
        }
        if let Some(fb) = FRAME_BUFFER.lock().as_mut() {
            fb.clear();
        }
    }
}

/// Console structure
pub struct Console<F: Font> {
    /// current color
    color: ColorPair<ConsoleColor>,
    /// cursor row
    row: usize,
    /// cursor column
    col: usize,
    /// character buffer
    buf: ConsoleBuffer<F>,
}

impl<F: Font> Console<F> {
    fn new(fb: &FramebufferInfo) -> Console<F> {
        let num_row = fb.yres as usize / F::HEIGHT;
        let num_col = fb.xres as usize / F::WIDTH;
        Console {
            color: ColorPair {
                foreground: BrightWhite,
                background: Black,
            },
            row: 0,
            col: 0,
            buf: ConsoleBuffer::new(num_row, num_col),
        }
    }

    fn new_line(&mut self) {
        self.col = 0;
        if self.row < self.buf.num_row - 1 {
            self.row += 1;
        } else {
            self.buf.new_line();
        }
    }

    // TODO: pasre color with ANSI escape sequences
    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\x7f' => {
                if self.col > 0 {
                    self.col -= 1;
                    self.buf.delete(self.row, self.col);
                } else if self.row > 0 {
                    self.row -= 1;
                    self.col = self.buf.num_col - 1;
                    self.buf.delete(self.row, self.col);
                }
            }
            b'\n' => self.new_line(),
            b'\r' => self.col = 0,
            byte => {
                if self.col >= self.buf.num_col {
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
        self.color = ColorPair {
            foreground: BrightWhite,
            background: Black,
        };
        self.row = 0;
        self.col = 0;
        self.buf.clear();
    }
}

impl<F: Font> fmt::Write for Console<F> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte)
        }
        Ok(())
    }
}

lazy_static! {
    pub static ref CONSOLE: Mutex<Option<Console<Font8x16>>> = Mutex::new(None);
}

/// Initialize console driver
pub fn init() {
    if let Some(fb) = FRAME_BUFFER.lock().as_ref() {
        *CONSOLE.lock() = Some(Console::new(&fb.fb_info));
    }

    if !CONSOLE.lock().is_none() {
        info!("console: init end");
    } else {
        warn!("console: init failed");
    }
}
