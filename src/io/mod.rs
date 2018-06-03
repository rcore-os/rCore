use arch::driver::serial::COM1;
use arch::driver::vga::Color;
use core::fmt;
use log;
use log::{Level, LevelFilter, Log, Metadata, Record};

mod vga_writer;

pub fn init() {
    static LOGGER: SimpleLogger = SimpleLogger;
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(match option_env!("LOG") {
        Some("off") => LevelFilter::Off,
        Some("error") => LevelFilter::Error,
        Some("warn") => LevelFilter::Warn,
        Some("info") => LevelFilter::Info,
        Some("trace") => LevelFilter::Trace,
        Some("debug") | _ => LevelFilter::Debug,
    });
}

macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::io::print(format_args!($($arg)*));
    });
}

macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

/// Add escape sequence to print with color in Linux console
macro_rules! with_color {
    ($args: ident, $color: ident) => {{
        let (show, code) = color_to_console_code($color);
        format_args!("{}[{};{}m{}{}[0m", 27 as char, show.clone(), code + 30, $args, 27 as char)
    }};
}

fn print_in_color(args: fmt::Arguments, color: Color) {
    use core::fmt::Write;
//    use arch::driver::vga::*;
//    {
//        let mut writer = vga_writer::VGA_WRITER.lock();
//        writer.set_color(color);
//        writer.write_fmt(args).unwrap();
//    }
    // TODO: 解决死锁问题
    // 若进程在持有锁时被中断，中断处理程序请求输出，就会死锁
    unsafe{ COM1.force_unlock(); }
    COM1.lock().write_fmt(with_color!(args, color)).unwrap();
}

pub fn print(args: fmt::Arguments) {
    use core::fmt::Write;
    unsafe { COM1.force_unlock(); }
    COM1.lock().write_fmt(args).unwrap();
}

struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        true
//        metadata.level() <= Level::Info
    }
    fn log(&self, record: &Record) {
        static DISABLED_TARGET: &[&str] = &[
            "rust_ucore::process::scheduler::rr",
        ];
        if self.enabled(record.metadata()) && !DISABLED_TARGET.contains(&record.target()) {
            print_in_color(format_args!("[{}] {}\n", record.target(), record.args()), Color::from(record.level()));
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
            Level::Debug => Color::LightRed,
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