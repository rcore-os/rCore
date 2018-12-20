//! Console font

mod font8x16;

pub use self::font8x16::Font8x16;

pub trait Font {
    const HEIGHT: usize;
    const WIDTH: usize;

    const UNDERLINE: usize;
    const STRIKETHROUGH: usize;

    /// Whether the character `byte` is visible at `(x, y)`.
    fn get(byte: u8, x: usize, y: usize) -> bool;
}
