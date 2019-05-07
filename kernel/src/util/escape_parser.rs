//! ANSI escape sequences parser
//! (ref: https://en.wikipedia.org/wiki/ANSI_escape_code)

#![allow(dead_code)]

use super::color::ConsoleColor;
use heapless::consts::U8;
use heapless::Vec;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharacterAttribute {
    /// foreground color
    pub foreground: ConsoleColor,
    /// background color
    pub background: ConsoleColor,
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
            foreground: ConsoleColor::White,
            background: ConsoleColor::Black,
            underline: false,
            reverse: false,
            strikethrough: false,
        }
    }
}

impl CharacterAttribute {
    /// Parse and apply SGR (Select Graphic Rendition) parameters.
    fn apply_sgr(&mut self, code: u8) {
        match code {
            0 => *self = CharacterAttribute::default(),
            4 => self.underline = true,
            7 => self.reverse = true,
            9 => self.strikethrough = true,
            24 => self.underline = false,
            27 => self.reverse = false,
            29 => self.strikethrough = false,
            30...37 | 90...97 => self.foreground = ConsoleColor::from_console_code(code).unwrap(),
            40...47 | 100...107 => {
                self.background = ConsoleColor::from_console_code(code - 10).unwrap()
            }
            _ => { /* unimplemented!() */ }
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CSI {
    CursorMove(i8, i8),
    CursorMoveLine(i8),
    SGR,
    Unknown,
}

impl CSI {
    fn new(final_byte: u8, params: &[u8]) -> CSI {
        let n = *params.get(0).unwrap_or(&1) as i8;
        match final_byte {
            b'A' => CSI::CursorMove(-n, 0),
            b'B' => CSI::CursorMove(n, 0),
            b'C' => CSI::CursorMove(0, n),
            b'D' => CSI::CursorMove(0, -n),
            b'E' => CSI::CursorMoveLine(n),
            b'F' => CSI::CursorMoveLine(-n),
            b'm' => CSI::SGR,
            _ => CSI::Unknown,
        }
    }
}

#[derive(Debug)]
pub struct EscapeParser {
    status: ParseStatus,
    char_attr: CharacterAttribute,
    current_param: Option<u8>,
    params: Vec<u8, U8>,
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
        assert_eq!(self.status, ParseStatus::Text);
        self.status = ParseStatus::BeginEscapeSequence;
        self.current_param = None;
    }

    /// See a character during parsing.
    /// Return `Some(csi)` if parse end, else `None`.
    pub fn parse(&mut self, byte: u8) -> Option<CSI> {
        assert_ne!(self.status, ParseStatus::Text);
        match self.status {
            ParseStatus::BeginEscapeSequence => match byte {
                b'[' => {
                    self.status = ParseStatus::ParsingCSI;
                    self.current_param = None;
                    self.params.clear();
                    return None;
                }
                _ => { /* unimplemented!() */ }
            },
            ParseStatus::ParsingCSI => match byte {
                b'0'...b'9' => {
                    let digit = (byte - b'0') as u32;
                    let param = self.current_param.unwrap_or(0) as u32;
                    let res = param * 10 + digit;
                    self.current_param = if res <= 0xFF { Some(res as u8) } else { None };
                    return None;
                }
                b';' => {
                    let param = self.current_param.unwrap_or(0);
                    self.params.push(param).unwrap();
                    self.current_param = Some(0);
                    return None;
                }
                // @A–Z[\]^_`a–z{|}~
                0x40...0x7E => {
                    if let Some(param) = self.current_param {
                        self.params.push(param).unwrap();
                    }
                    let csi = CSI::new(byte, &self.params);
                    if csi == CSI::SGR {
                        if self.params.is_empty() {
                            self.char_attr.apply_sgr(0);
                        } else {
                            for &param in self.params.iter() {
                                self.char_attr.apply_sgr(param);
                            }
                        }
                    }
                    self.status = ParseStatus::Text;
                    self.current_param = None;
                    self.params.clear();
                    return Some(csi);
                }
                _ => {}
            },
            ParseStatus::Text => {}
        }
        self.status = ParseStatus::Text;
        self.current_param = None;
        self.params.clear();
        None
    }

    pub fn char_attribute(&self) -> CharacterAttribute {
        self.char_attr
    }
}
