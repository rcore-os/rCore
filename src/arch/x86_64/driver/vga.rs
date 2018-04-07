use spin::Mutex;
use core::ptr::Unique;
use volatile::Volatile;
use x86_64::instructions::port::{outw, outb};

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
            outw(0x3D4, 15u16); // WARNING verify should be u16
            outb(0x3D5, pos as u8);
            outw(0x3D4, 14u16);
            outb(0x3D5, (pos >> 8) as u8);
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn print_something() {
        let vga = unsafe {&mut *VGA_BUFFER};
        vga.clear();
        vga.write(0, 0, ScreenChar::new('h', Color::LightGray, Color::Black));
    }
}