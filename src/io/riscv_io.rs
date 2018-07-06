// FIXME: merge to x86_64 io

use core::fmt;

macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::io::print(format_args!($($arg)*));
    });
}

macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

pub fn print(args: fmt::Arguments) {
    use arch::serial::SerialPort;
    use core::fmt::Write;
    SerialPort.write_fmt(args).unwrap();
}