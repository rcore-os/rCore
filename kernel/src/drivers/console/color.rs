//! Frambuffer color

use crate::util::color::ConsoleColor;

use super::ColorConfig;

pub trait FramebufferColor {
    fn pack32(&self, config: ColorConfig) -> u32;
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
    fn pack32(&self, config: ColorConfig) -> u32 {
        match config {
            ColorConfig::RGB332 => {
                (((self.0 >> 5) << 5) | ((self.1 >> 5) << 2) | (self.2 >> 6)) as u32
            }
            ColorConfig::RGB565 => {
                (((self.0 as u16 & 0xF8) << 8)
                    | ((self.1 as u16 & 0xFC) << 3)
                    | (self.2 as u16 >> 3)) as u32
            }
            // FIXME: qemu and low version RPi use RGBA order for 24/32-bit color depth,
            // but RPi3 B+ uses BGRA order for 24/32-bit color depth.
            ColorConfig::BGRA8888 => {
                ((self.0 as u32) << 16) | ((self.1 as u32) << 8) | (self.2 as u32)
            }
            _ => unimplemented!(),
        }
    }
}

impl FramebufferColor for ConsoleColor {
    #[inline]
    fn pack32(&self, config: ColorConfig) -> u32 {
        match config {
            ColorConfig::VgaPalette => *self as u32,
            _ => RgbColor::from(*self).pack32(config),
        }
    }
}
