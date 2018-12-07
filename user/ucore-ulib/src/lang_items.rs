use crate::syscall::{sys_close, sys_dup, sys_exit, sys_open};
use crate::syscall::{O_RDONLY, O_WRONLY};
use core::alloc::Layout;
use core::panic::PanicInfo;

// used for panic
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::syscall::print_putc(format_args!($($arg)*));
    });
}

#[linkage = "weak"]
#[no_mangle]
fn main() {
    panic!("No main() linked");
}

fn initfd(fd2: usize, path: &str, open_flags: usize) -> i32 {
    let fd1 = sys_open(path, open_flags);
    if fd1 < 0 {
        return fd1;
    }
    let mut ret = fd1;
    let fd1 = fd1 as usize;
    if fd1 != fd2 {
        sys_close(fd2);
        ret = sys_dup(fd1, fd2);
        sys_close(fd1);
    }
    return ret;
}

#[no_mangle]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> ! {
    let fd = initfd(0, "stdin:", O_RDONLY);
    if fd < 0 {
        panic!("open <stdin> failed: {}.", fd);
    }
    let fd = initfd(1, "stdout:", O_WRONLY);
    if fd < 0 {
        panic!("open <stdout> failed: {}.", fd);
    }

    main();
    sys_exit(0)
}

#[lang = "eh_personality"]
fn eh_personality() {}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let location = info.location().unwrap();
    let message = info.message().unwrap();
    println!("\n\nPANIC in {} at line {}\n    {}", location.file(), location.line(), message);
    sys_exit(1)
}

#[lang = "oom"]
fn oom(_: Layout) -> ! {
    panic!("out of memory");
}

#[no_mangle]
pub extern "C" fn abort() -> ! {
    sys_exit(2)
}

#[no_mangle]
pub extern "C" fn __mulsi3(mut a: u32, mut b: u32) -> u32 {
    let mut r: u32 = 0;

    while a > 0 {
        if a & 1 > 0 {
            r += b;
        }
        a >>= 1;
        b <<= 1;
    }

    r
}
