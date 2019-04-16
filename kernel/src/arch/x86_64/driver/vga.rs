use core::fmt;

use console_traits::*;
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;
use x86_64::instructions::port::Port;

use crate::consts::KERNEL_OFFSET;
use crate::util::color::ConsoleColor;
use crate::util::escape_parser::{EscapeParser, CSI};

#[derive(Debug, Clone, Copy)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: ConsoleColor, background: ConsoleColor) -> ColorCode {
        ColorCode(background.to_code() << 4 | foreground.to_code())
    }
}

impl ConsoleColor {
    fn to_code(&self) -> u8 {
        use self::ConsoleColor::*;
        match self {
            Black => 0,
            Blue => 1,
            Green => 2,
            Cyan => 3,
            Red => 4,
            Magenta => 5,
            Yellow => 6,
            White => 7,
            BrightBlack => 8,
            BrightBlue => 9,
            BrightGreen => 10,
            BrightCyan => 11,
            BrightRed => 12,
            BrightMagenta => 13,
            BrightYellow => 14,
            BrightWhite => 15,
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ScreenChar {
    ascii_char: u8,
    color_code: ColorCode,
}

impl ScreenChar {
    pub fn new(
        ascii_char: u8,
        foreground_color: ConsoleColor,
        background_color: ConsoleColor,
    ) -> Self {
        ScreenChar {
            ascii_char,
            color_code: ColorCode::new(foreground_color, background_color),
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
        let blank = ScreenChar::new(b' ', ConsoleColor::White, ConsoleColor::Black);
        for row in 0..BUFFER_HEIGHT {
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
    color_code: ColorCode,
    ctrl_char_mode: ControlCharMode,
    esc_char_mode: EscapeCharMode,
    escape_parser: EscapeParser,
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

    fn set_pos(&mut self, mut pos: Position) -> Result<(), Self::Error> {
        pos.row.bound(self.get_height());
        pos.col.bound(self.get_width());
        self.pos = pos;
        self.buffer
            .set_cursor_at(pos.row.0 as usize, pos.col.0 as usize);
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
        let blank = ScreenChar::new(b' ', ConsoleColor::White, ConsoleColor::Black);
        for col in 0..BUFFER_WIDTH {
            self.buffer.write(BUFFER_HEIGHT - 1, col, blank);
        }
        Ok(())
    }
}

impl AsciiConsole for VgaWriter {
    fn write_char_at(&mut self, ch: u8, pos: Position) -> Result<(), Self::Error> {
        let screen_char = ScreenChar {
            ascii_char: ch,
            color_code: self.color_code,
        };
        self.buffer
            .write(pos.row.0 as usize, pos.col.0 as usize, screen_char);
        Ok(())
    }

    fn handle_escape(&mut self, escaped_char: u8) -> bool {
        if escaped_char == b'[' {
            self.escape_parser.start_parse();
        }
        let csi = match self.escape_parser.parse(escaped_char) {
            Some(csi) => csi,
            None => return false,
        };
        match csi {
            CSI::SGR => {
                let attr = self.escape_parser.char_attribute();
                self.color_code = ColorCode::new(attr.foreground, attr.background);
            }
            CSI::CursorMove(dx, dy) => {
                let x = (self.pos.row.0 as i8 + dx).max(0) as u8;
                let y = (self.pos.col.0 as i8 + dy).max(0) as u8;
                self.set_pos(Position::new(Row(x), Col(y))).unwrap();
            }
            CSI::CursorMoveLine(dx) => {
                let x = (self.pos.row.0 as i8 + dx).max(0) as u8;
                self.set_pos(Position::new(Row(x), Col(0))).unwrap();
            }
            _ => {}
        }
        true
    }

    /// Check if an 8-bit char is special
    fn is_special(&self, ch: u8) -> Option<SpecialChar> {
        match self.get_control_char_mode() {
            ControlCharMode::Interpret => match ch {
                b'\n' => Some(SpecialChar::Linefeed),
                b'\r' => Some(SpecialChar::CarriageReturn),
                b'\t' => Some(SpecialChar::Tab),
                0x1b => Some(SpecialChar::Escape),
                0x7f => Some(SpecialChar::Delete),
                0x08 => Some(SpecialChar::Backspace),
                _ if !(ch.is_ascii_graphic() || ch == b' ') => Some(SpecialChar::Delete), // ignore non-graphic ascii
                _ => None,
            },
            _ => None,
        }
    }
}

impl VgaWriter {
    fn new(buffer: &'static mut VgaBuffer) -> Self {
        buffer.clear();
        VgaWriter {
            pos: Position::origin(),
            color_code: ColorCode::new(ConsoleColor::White, ConsoleColor::Black),
            ctrl_char_mode: ControlCharMode::Interpret,
            esc_char_mode: EscapeCharMode::Waiting,
            escape_parser: EscapeParser::new(),
            buffer,
        }
    }
}

impl fmt::Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s.as_bytes()).map_err(|_| fmt::Error)
    }
}
