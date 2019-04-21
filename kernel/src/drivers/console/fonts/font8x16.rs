//! CP437 8x16 dot matrix font

use super::Font;

pub enum Font8x16 {}

// https://github.com/jamwaffles/embedded-graphics/blob/0ec4fc09c55aee733fb9d9cd6525749b8e15766e/embedded-graphics/data/font8x16_1bpp.raw
const FONT_IMAGE: &'static [u8] = include_bytes!("font8x16_1bpp.raw");

impl Font for Font8x16 {
    const HEIGHT: usize = 16;
    const WIDTH: usize = 8;

    const UNDERLINE: usize = 13;
    const STRIKETHROUGH: usize = 8;

    #[inline]
    fn get(byte: u8, x: usize, y: usize) -> bool {
        const FONT_IMAGE_WIDTH: usize = 240;
        let char_per_row = FONT_IMAGE_WIDTH / Self::WIDTH;

        // Char _code_ offset from first char, most often a space
        // E.g. first char = ' ' (32), target char = '!' (33), offset = 33 - 32 = 1
        let char_offset = char_offset(byte as char) as usize;
        let row = char_offset / char_per_row;

        // Top left corner of character, in pixels
        let char_x = (char_offset - (row * char_per_row)) * Self::WIDTH;
        let char_y = row * Self::HEIGHT;

        // Bit index
        // = X pixel offset for char
        // + Character row offset (row 0 = 0, row 1 = (192 * 8) = 1536)
        // + X offset for the pixel block that comprises this char
        // + Y offset for pixel block
        let bitmap_bit_index = char_x + x + (FONT_IMAGE_WIDTH * (char_y + y));

        let bitmap_byte = bitmap_bit_index / 8;
        let bitmap_bit = 7 - (bitmap_bit_index % 8);

        FONT_IMAGE[bitmap_byte] & ((1 << bitmap_bit) as u8) != 0
    }
}

fn char_offset(c: char) -> u32 {
    let fallback = '?' as u32 - ' ' as u32;
    if c < ' ' {
        return fallback;
    }
    if c <= '~' {
        return c as u32 - ' ' as u32;
    }
    if c < '¡' || c > 'ÿ' {
        return fallback;
    }
    c as u32 - ' ' as u32 - 34
}
