use core::fmt;
use arch::driver::serial::COM1;

mod vga_writer;

macro_rules! _debug {
    ($($arg:tt)*) => ({
        $crate::io::debug(format_args!($($arg)*));
    });
}

macro_rules! debug {
    ($fmt:expr) => (_debug!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (_debug!(concat!($fmt, "\n"), $($arg)*));
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

use arch::driver::vga::Color;

fn print_in_color(args: fmt::Arguments, color: Color) {
    use core::fmt::Write;
    use arch::driver::vga::*;
//    {
//        let mut writer = vga_writer::VGA_WRITER.lock();
//        writer.set_color(color);
//        writer.write_fmt(args).unwrap();
//    }
    COM1.lock().write_fmt(args).unwrap();
}

pub fn print(args: fmt::Arguments) {
    print_in_color(args, Color::LightGray);
}

pub fn debug(args: fmt::Arguments) {
    print_in_color(args, Color::LightRed);
}