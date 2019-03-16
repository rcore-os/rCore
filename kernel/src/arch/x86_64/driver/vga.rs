use core::fmt;
use spin::Mutex;
use volatile::Volatile;
use lazy_static::lazy_static;
use x86_64::instructions::port::Port;
use crate::logging::Color;
use crate::consts::KERNEL_OFFSET;
use console_traits::*;

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
	    // VGA virtual address is specified at bootloader
		VgaWriter::new(unsafe{ &mut *((KERNEL_OFFSET + 0xf0000000) as *mut VgaBuffer) })
	);
}

pub struct VgaWriter {
    pos: Position,
    color: Color,
    ctrl_char_mode: ControlCharMode,
    esc_char_mode: EscapeCharMode,
    buffer: &'static mut VgaBuffer,
}

impl BaseConsole for VgaWriter {
    type Error = ();

    fn get_width(&self) -> Col {
        Col(BUFFER_WIDTH as u8 - 1)
    }

    fn get_height(&self) -> Row {
        Row(BUFFER_HEIGHT as u8 - 1)
    }

    fn set_col(&mut self, col: Col) -> Result<(), Self::Error> {
        self.pos.col = col;
        Ok(())
    }

    fn set_row(&mut self, row: Row) -> Result<(), Self::Error> {
        self.pos.row = row;
        Ok(())
    }

    fn set_pos(&mut self, pos: Position) -> Result<(), Self::Error> {
        self.pos = pos;
        Ok(())
    }

    fn get_pos(&self) -> Position {
        self.pos
    }

    fn set_control_char_mode(&mut self, mode: ControlCharMode) {
        self.ctrl_char_mode = mode;
    }

    fn get_control_char_mode(&self) -> ControlCharMode {
        self.ctrl_char_mode
    }

    fn set_escape_char_mode(&mut self, mode: EscapeCharMode) {
        self.esc_char_mode = mode;
    }

    fn get_escape_char_mode(&self) -> EscapeCharMode {
        self.esc_char_mode
    }

    fn scroll_screen(&mut self) -> Result<(), Self::Error> {
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
        self.buffer.set_cursor_at(BUFFER_HEIGHT - 1, 0);
        Ok(())
    }
}

impl AsciiConsole for VgaWriter {
    fn write_char_at(&mut self, ch: u8, pos: Position) -> Result<(), Self::Error> {
        self.buffer.write(pos.row.0 as usize, pos.col.0 as usize, ScreenChar::new(ch, self.color, Color::Black));
        self.buffer.set_cursor_at(pos.row.0 as usize, pos.col.0 as usize);
        Ok(())
    }

    fn handle_escape(&mut self, escaped_char: u8) -> bool {
        true
    }
}

impl VgaWriter {
    fn new(buffer: &'static mut VgaBuffer) -> Self {
        buffer.clear();
        VgaWriter {
            pos: Position::origin(),
            color: Color::LightGray,
            ctrl_char_mode: ControlCharMode::Interpret,
            esc_char_mode: EscapeCharMode::Waiting,
            buffer,
        }
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }
}

impl fmt::Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s.as_bytes())
            .map_err(|_| fmt::Error)
    }
}
