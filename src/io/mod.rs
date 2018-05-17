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
    // TODO: 解决死锁问题
    // 若进程在持有锁时被中断，中断处理程序请求输出，就会死锁
    unsafe{ COM1.force_unlock(); }
    COM1.lock().write_fmt(args).unwrap();
}

pub fn print(args: fmt::Arguments) {
    print_in_color(args, Color::LightGray);
}

pub fn debug(args: fmt::Arguments) {
    print_in_color(args, Color::LightRed);
}

pub fn write(fd: usize, base: *const u8, len: usize) -> i32 {
    debug!("write: fd: {}, base: {:?}, len: {:#x}", fd, base, len);
    use core::slice;
    use core::str;
    let slice = unsafe { slice::from_raw_parts(base, len) };
    print!("{}", str::from_utf8(slice).unwrap());
    0
}

pub fn open(path: *const u8, flags: usize) -> i32 {
    let path = unsafe { from_cstr(path) };
    debug!("open: path: {:?}, flags: {:?}", path, flags);
    match path {
        "stdin:" => 0,
        "stdout:" => 1,
        _ => -1,
    }
}

pub fn close(fd: usize) -> i32 {
    debug!("close: fd: {:?}", fd);
    0
}

pub unsafe fn from_cstr(s: *const u8) -> &'static str {
    use core::{str, slice};
    let len = (0usize..).find(|&i| *s.offset(i as isize) == 0).unwrap();
    str::from_utf8(slice::from_raw_parts(s, len)).unwrap()
}