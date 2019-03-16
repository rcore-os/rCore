//! Frambuffer color

use crate::util::color::ConsoleColor;

pub trait FramebufferColor {
    /// pack as 16-bit integer
    fn pack16(&self) -> u16;

    /// pack as 32-bit integer
    fn pack32(&self) -> u32;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbColor(u8, u8, u8);

impl From<ConsoleColor> for RgbColor {
    /// Convert `ConsoleColor` to `RgbColor`.
    /// use `CMD` color scheme.
    /// (ref: https://en.wikipedia.org/wiki/ANSI_escape_code)
    fn from(color: ConsoleColor) -> Self {
        use self::ConsoleColor::*;
        match color {
            Black => RgbColor(0, 0, 0),
            Red => RgbColor(128, 0, 0),
            Green => RgbColor(0, 128, 8),
            Yellow => RgbColor(128, 128, 0),
            Blue => RgbColor(0, 0, 128),
            Magenta => RgbColor(128, 0, 128),
            Cyan => RgbColor(0, 128, 128),
            White => RgbColor(192, 192, 192),
            BrightBlack => RgbColor(128, 128, 128),
            BrightRed => RgbColor(255, 0, 0),
            BrightGreen => RgbColor(0, 255, 0),
            BrightYellow => RgbColor(255, 255, 0),
            BrightBlue => RgbColor(0, 0, 255),
            BrightMagenta => RgbColor(255, 0, 255),
            BrightCyan => RgbColor(0, 255, 255),
            BrightWhite => RgbColor(255, 255, 255),
        }
    }
}

impl FramebufferColor for RgbColor {
    #[inline]
    fn pack16(&self) -> u16 {
        // BGR565
        ((self.0 as u16 & 0xF8) << 8) | ((self.1 as u16 & 0xFC) << 3) | (self.2 as u16 >> 3)
    }

    #[inline]
    fn pack32(&self) -> u32 {
        // BGRA8888
        // FIXME: qemu and low version RPi use RGBA order for 24/32-bit color depth,
        // but RPi3 B+ uses BGRA order for 24/32-bit color depth.
        ((self.0 as u32) << 16) | ((self.1 as u32) << 8) | (self.2 as u32)
    }
}

impl FramebufferColor for ConsoleColor {
    #[inline]
    fn pack16(&self) -> u16 {
        RgbColor::from(*self).pack16()
    }

    #[inline]
    fn pack32(&self) -> u32 {
        RgbColor::from(*self).pack32()
    }
}
