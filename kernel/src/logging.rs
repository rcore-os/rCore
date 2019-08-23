use core::fmt;

use lazy_static::lazy_static;
use log::{self, Level, LevelFilter, Log, Metadata, Record};

use crate::processor;
use crate::sync::SpinNoIrqLock as Mutex;

lazy_static! {
    static ref LOG_LOCK: Mutex<()> = Mutex::new(());
}

pub fn init() {
    static LOGGER: SimpleLogger = SimpleLogger;
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(match option_env!("LOG") {
        Some("error") => LevelFilter::Error,
        Some("warn") => LevelFilter::Warn,
        Some("info") => LevelFilter::Info,
        Some("debug") => LevelFilter::Debug,
        Some("trace") => LevelFilter::Trace,
        _ => LevelFilter::Off,
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
    ($args: ident, $color_code: ident) => {{
        format_args!("\u{1B}[{}m{}\u{1B}[0m", $color_code as u8, $args)
    }};
}

fn print_in_color(args: fmt::Arguments, color_code: u8) {
    use crate::arch::io;
    let _guard = LOG_LOCK.lock();
    io::putfmt(with_color!(args, color_code));
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
        if !self.enabled(record.metadata()) {
            return;
        }

        if let Some(tid) = processor().tid_option() {
            print_in_color(
                format_args!(
                    "[{:>5}][{}][{}] {}\n",
                    record.level(),
                    tid,
                    record.target(),
                    record.args()
                ),
                level_to_color_code(record.level()),
            );
        } else {
            print_in_color(
                format_args!(
                    "[{:>5}][-][{}] {}\n",
                    record.level(),
                    record.target(),
                    record.args()
                ),
                level_to_color_code(record.level()),
            );
        }
    }
    fn flush(&self) {}
}

fn level_to_color_code(level: Level) -> u8 {
    match level {
        Level::Error => 31, // Red
        Level::Warn => 93,  // BrightYellow
        Level::Info => 34,  // Blue
        Level::Debug => 32, // Green
        Level::Trace => 90, // BrightBlack
    }
}
