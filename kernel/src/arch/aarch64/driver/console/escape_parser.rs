//! ANSI escape sequences parser
//! (ref: https://en.wikipedia.org/wiki/ANSI_escape_code)

use super::color::{ConsoleColor, ConsoleColor::*, FramebufferColor};
use alloc::vec::Vec;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharacterAttribute<C: FramebufferColor = ConsoleColor> {
    /// foreground color
    pub foreground: C,
    /// background color
    pub background: C,
    /// show underline
    pub underline: bool,
    /// swap foreground and background colors
    pub reverse: bool,
    /// text marked cfor deletion
    pub strikethrough: bool,
}

impl Default for CharacterAttribute {
    fn default() -> Self {
        CharacterAttribute {
            foreground: White,
            background: Black,
            underline: false,
            reverse: false,
            strikethrough: false,
        }
    }
}

#[derive(Debug, PartialEq)]
enum ParseStatus {
    /// The last character is `ESC`, start parsing the escape sequence.
    BeginEscapeSequence,

    /// The character followed by `ESC` is `[`, start parsing the CSI (Control
    /// Sequence Introducer) sequence. The CSI sequence format is like
    /// `ESC [ n1 ; n2 ; ... m`.
    ParsingCSI,

    /// Display text Normally.
    Text,
}

#[derive(Debug)]
pub struct EscapeParser {
    status: ParseStatus,
    char_attr: CharacterAttribute,
    current_param: Option<u8>,
    params: Vec<u8>,
}

impl EscapeParser {
    pub fn new() -> EscapeParser {
        EscapeParser {
            status: ParseStatus::Text,
            char_attr: CharacterAttribute::default(),
            params: Vec::new(),
            current_param: None,
        }
    }

    pub fn is_parsing(&self) -> bool {
        self.status != ParseStatus::Text
    }

    /// See an `ECS` character, start parsing escape sequence.
    pub fn start_parse(&mut self) {
        assert!(self.status == ParseStatus::Text);
        self.status = ParseStatus::BeginEscapeSequence;
        self.current_param = None;
    }

    //// Parse SGR (Select Graphic Rendition) parameters.
    fn parse_sgr_params(&mut self) {
        use core::mem::transmute;
        for param in &self.params {
            match param {
                0 => self.char_attr = CharacterAttribute::default(),
                4 => self.char_attr.underline = true,
                7 => self.char_attr.reverse = true,
                9 => self.char_attr.strikethrough = true,
                24 => self.char_attr.underline = false,
                27 => self.char_attr.reverse = false,
                29 => self.char_attr.strikethrough = false,
                30...37 | 90...97 => self.char_attr.foreground = unsafe { transmute(param - 30) },
                40...47 | 100...107 => self.char_attr.background = unsafe { transmute(param - 40) },
                _ => { /* unimplemented!() */ }
            }
        }
    }

    /// See a character during parsing.
    pub fn parse(&mut self, byte: u8) -> bool {
        assert!(self.status != ParseStatus::Text);
        match self.status {
            ParseStatus::BeginEscapeSequence => match byte {
                b'[' => {
                    self.status = ParseStatus::ParsingCSI;
                    self.current_param = Some(0);
                    self.params.clear();
                    return true;
                }
                _ => { /* unimplemented!() */ }
            },
            ParseStatus::ParsingCSI => match byte {
                b'0'...b'9' => {
                    let digit = (byte - b'0') as u32;
                    if let Some(param) = self.current_param {
                        let res: u32 = param as u32 * 10 + digit;
                        self.current_param = if res <= 0xFF { Some(res as u8) } else { None };
                    }
                    return true;
                }
                b';' => {
                    if let Some(param) = self.current_param {
                        self.params.push(param);
                    }
                    self.current_param = Some(0);
                    return true;
                }
                // @A–Z[\]^_`a–z{|}~
                0x40...0x7E => {
                    if let Some(param) = self.current_param {
                        self.params.push(param);
                    }
                    match byte {
                        b'm' => self.parse_sgr_params(),
                        _ => { /* unimplemented!() */ }
                    }
                    self.status = ParseStatus::Text;
                    self.current_param = None;
                    self.params.clear();
                    return true;
                }
                _ => {}
            },
            ParseStatus::Text => {}
        }
        self.status = ParseStatus::Text;
        self.current_param = None;
        self.params.clear();
        false
    }

    pub fn char_attribute(&self) -> CharacterAttribute {
        self.char_attr
    }
}
