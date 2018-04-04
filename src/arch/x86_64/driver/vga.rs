use spin::Mutex;
use core::ptr::Unique;
use volatile::Volatile;

pub const VGA_BUFFER: Unique<VgaBuffer> = unsafe{ 
    Unique::new_unchecked(0xb8000 as *mut _) 
};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Color {
    Black      = 0,
    Blue       = 1,
    Green      = 2,
    Cyan       = 3,
    Red        = 4,
    Magenta    = 5,
    Brown      = 6,
    LightGray  = 7,
    DarkGray   = 8,
    LightBlue  = 9,
    LightGreen = 10,
    LightCyan  = 11,
    LightRed   = 12,
    Pink       = 13,
    Yellow     = 14,
    White      = 15,
}

#[derive(Debug, Clone, Copy)]
struct ColorCode(u8);

impl ColorCode {
    const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct ScreenChar {
    ascii_char: u8,
    color_code: ColorCode,
}

pub const BUFFER_HEIGHT: usize = 25;
pub const BUFFER_WIDTH: usize = 80;

pub struct VgaBuffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

impl VgaBuffer {
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_char: b' ',
            color_code: ColorCode::new(Color::White, Color::Black),
        };
        for col in 0..BUFFER_WIDTH {
            self.chars[row][col].write(blank);
        }
    }
    pub fn clear(&mut self) {
        for i in 0 .. BUFFER_HEIGHT {
            self.clear_row(i);
        }
    }
    pub fn write(&mut self, row: usize, col: usize, byte: u8) {
        let screen_char = &mut self.chars[row][col];
        let color_code = screen_char.read().color_code;
        screen_char.write(ScreenChar {
            ascii_char: byte,
            color_code: color_code,
        });
    }
    pub fn byte_at(&self, row: usize, col: usize) -> u8 {
        self.chars[row][col].read().ascii_char
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn print_something() {
        let vga = unsafe {&mut *VGA_BUFFER};
        vga.clear();
        vga.write(0, 0, b'a');
    }
}