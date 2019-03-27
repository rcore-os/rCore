use core::fmt;

use lazy_static::lazy_static;
use log::{self, Level, LevelFilter, Log, Metadata, Record};

use crate::sync::SpinNoIrqLock as Mutex;
use crate::util::color::ConsoleColor;

lazy_static! {
    static ref LOG_LOCK: Mutex<()> = Mutex::new(());
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
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

/// Add escape sequence to print with color in Linux console
macro_rules! with_color {
    ($args: ident, $color: ident) => {{
        let code = $color.to_console_code();
        format_args!("\u{1B}[{}m{}\u{1B}[0m", code as u8, $args)
    }};
}

fn print_in_color(args: fmt::Arguments, color: ConsoleColor) {
    use crate::arch::io;
    let _guard = LOG_LOCK.lock();
    io::putfmt(with_color!(args, color));
}

pub fn print(args: fmt::Arguments) {
    use crate::arch::io;
    let _guard = LOG_LOCK.lock();
    io::putfmt(args);
}

struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }
    fn log(&self, record: &Record) {
        static DISABLED_TARGET: &[&str] = &[];
        if self.enabled(record.metadata()) && !DISABLED_TARGET.contains(&record.target()) {
            //            let target = record.target();
            //            let begin = target.as_bytes().iter().rposition(|&c| c == b':').map(|i| i + 1).unwrap_or(0);
            print_in_color(
                format_args!("[{:>5}] {}\n", record.level(), record.args()),
                ConsoleColor::from(record.level()),
            );
        }
    }
    fn flush(&self) {}
}

impl From<Level> for ConsoleColor {
    fn from(level: Level) -> Self {
        match level {
            Level::Error => ConsoleColor::Red,
            Level::Warn => ConsoleColor::BrightYellow,
            Level::Info => ConsoleColor::Blue,
            Level::Debug => ConsoleColor::Green,
            Level::Trace => ConsoleColor::BrightBlack,
        }
    }
}
