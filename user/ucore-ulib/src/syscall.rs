use core::fmt::{self, Write};

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::syscall::print(format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

pub fn print(args: fmt::Arguments) {
    StdOut.write_fmt(args).unwrap();
}

struct StdOut;

impl fmt::Write for StdOut {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match sys_write(0, s.as_ptr(), s.len()) {
            0 => Ok(()),
            _ => Err(fmt::Error::default()),
        }
    }
}

#[inline(always)]
fn sys_call(id: usize, arg0: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> isize {
    let ret: isize;
    unsafe {
        #[cfg(target_arch = "riscv")]
            asm!("ecall"
            : "={x10}" (ret)
            : "{x17}" (id), "{x10}" (arg0), "{x11}" (arg1), "{x12}" (arg2), "{x13}" (arg3), "{x14}" (arg4), "{x15}" (arg5)
            : "memory"
            : "volatile");
        #[cfg(target_arch = "x86_64")]
            asm!("int 0x80"
            : "={rax}" (ret)
            : "{rax}" (id), "{rdi}" (arg0), "{rsi}" (arg1), "{rdx}" (arg2), "{rcx}" (arg3), "{r8}" (arg4), "{r9}" (arg5)
            : "memory"
            : "intel" "volatile");
    }
    ret
}

pub fn sys_exit(code: usize) -> ! {
    sys_call(SYS_EXIT, code, 0, 0, 0, 0, 0);
    unreachable!()
}

pub fn sys_write(fd: usize, base: *const u8, len: usize) -> isize {
    sys_call(SYS_WRITE, fd, base as usize, len, 0, 0, 0)
}

const SYS_EXIT: usize = 1;
const SYS_FORK: usize = 2;
const SYS_WAIT: usize = 3;
const SYS_EXEC: usize = 4;
const SYS_CLONE: usize = 5;
const SYS_YIELD: usize = 10;
const SYS_SLEEP: usize = 11;
const SYS_KILL: usize = 12;
const SYS_GETTIME: usize = 17;
const SYS_GETPID: usize = 18;
const SYS_MMAP: usize = 20;
const SYS_MUNMAP: usize = 21;
const SYS_SHMEM: usize = 22;
const SYS_PUTC: usize = 30;
const SYS_PGDIR: usize = 31;
const SYS_OPEN: usize = 100;
const SYS_CLOSE: usize = 101;
const SYS_READ: usize = 102;
const SYS_WRITE: usize = 103;
const SYS_SEEK: usize = 104;
const SYS_FSTAT: usize = 110;
const SYS_FSYNC: usize = 111;
const SYS_GETCWD: usize = 121;
const SYS_GETDIRENTRY: usize = 128;
const SYS_DUP: usize = 130;
const SYS_LAB6_SET_PRIORITY: usize = 255;