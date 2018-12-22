use core::fmt;
use log::{self, Level, LevelFilter, Log, Metadata, Record};
use crate::sync::SpinNoIrqLock as Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref log_mutex: Mutex<()> = Mutex::new(());
}

pub fn init() {
    static LOGGER: SimpleLogger = SimpleLogger;
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(match option_env!("LOG") {
        Some("off") => LevelFilter::Off,
        Some("error") => LevelFilter::Error,
        Some("warn") => LevelFilter::Warn,
        Some("info") => LevelFilter::Info,
        Some("debug") => LevelFilter::Debug,
        Some("trace") => LevelFilter::Trace,
        _ => LevelFilter::Warn,
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::logging::print(format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\r\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\r\n"), $($arg)*));
}

/// Add escape sequence to print with color in Linux console
macro_rules! with_color {
    ($args: ident, $color: ident) => {{
        let (show, code) = color_to_console_code($color);
        format_args!("\u{1B}[{};{}m{}\u{1B}[0m", show.clone(), code + 30, $args)
    }};
}

fn print_in_color(args: fmt::Arguments, color: Color) {
    use crate::arch::io;
    let mutex = log_mutex.lock();
    io::putfmt(with_color!(args, color));
}

pub fn print(args: fmt::Arguments) {
    use crate::arch::io;
    let mutex = log_mutex.lock();
    io::putfmt(args);
}

struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        true
//        metadata.level() <= Level::Info
    }
    fn log(&self, record: &Record) {
        static DISABLED_TARGET: &[&str] = &[
        ];
        if self.enabled(record.metadata()) && !DISABLED_TARGET.contains(&record.target()) {
//            let target = record.target();
//            let begin = target.as_bytes().iter().rposition(|&c| c == b':').map(|i| i + 1).unwrap_or(0);
            print_in_color(format_args!("[{:>5}] {}\r\n", record.level(), record.args()), Color::from(record.level()));
        }
    }
    fn flush(&self) {}
}

impl From<Level> for Color {
    fn from(level: Level) -> Self {
        match level {
            Level::Error => Color::Red,
            Level::Warn => Color::Yellow,
            Level::Info => Color::Blue,
            Level::Debug => Color::Green,
            Level::Trace => Color::DarkGray,
        }
    }
}

fn color_to_console_code(color: Color) -> (u8, u8) {
    match color {
        Color::Black => (0, 0),
        Color::Blue => (0, 4),
        Color::Green => (0, 2),
        Color::Cyan => (0, 6),
        Color::Red => (0, 1),
        Color::Magenta => (0, 5),
        Color::Brown => (0, 3),
        Color::LightGray => (1, 7),
        Color::DarkGray => (0, 7),
        Color::LightBlue => (1, 4),
        Color::LightGreen => (1, 2),
        Color::LightCyan => (1, 6),
        Color::LightRed => (1, 1),
        Color::Pink => (1, 5),
        Color::Yellow => (1, 3),
        Color::White => (1, 0),
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}