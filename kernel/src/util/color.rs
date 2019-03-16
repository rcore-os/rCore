#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ConsoleColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl ConsoleColor {
    pub fn to_console_code(&self) -> u8 {
        use self::ConsoleColor::*;
        match self {
            Black => 30,
            Red => 31,
            Green => 32,
            Yellow => 33,
            Blue => 34,
            Magenta => 35,
            Cyan => 36,
            White => 37,
            BrightBlack => 90,
            BrightRed => 91,
            BrightGreen => 92,
            BrightYellow => 93,
            BrightBlue => 94,
            BrightMagenta => 95,
            BrightCyan => 96,
            BrightWhite => 97,
        }
    }
    pub fn from_console_code(code: u8) -> Option<ConsoleColor> {
        use self::ConsoleColor::*;
        match code {
            30 => Some(Black),
            31 => Some(Red),
            32 => Some(Green),
            33 => Some(Yellow),
            34 => Some(Blue),
            35 => Some(Magenta),
            36 => Some(Cyan),
            37 => Some(White),
            90 => Some(BrightBlack),
            91 => Some(BrightRed),
            92 => Some(BrightGreen),
            93 => Some(BrightYellow),
            94 => Some(BrightBlue),
            95 => Some(BrightMagenta),
            96 => Some(BrightCyan),
            97 => Some(BrightWhite),
            _ => None,
        }
    }
}
