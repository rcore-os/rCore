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

pub fn print_putc(args: fmt::Arguments) {
    SysPutc.write_fmt(args).unwrap();
}

struct StdOut;
struct SysPutc;

impl fmt::Write for StdOut {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if sys_write(1, s.as_ptr(), s.len()) >= 0 {
            Ok(())
        } else {
            Err(fmt::Error::default())
        }
    }
}

impl fmt::Write for SysPutc {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.bytes() {
            sys_putc(c as char);
        }
        Ok(())
    }
}

#[inline(always)]
fn sys_call(id: usize, arg0: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> i32 {
    #[cfg(target_arch = "riscv64")]
    let ret: i32 = 0;
    #[cfg(target_arch = "riscv32")]
    let ret: i32;

    unsafe {
        #[cfg(target_arch = "riscv32")]
            asm!("ecall"
            : "={x10}" (ret)
            : "{x10}" (id), "{x11}" (arg0), "{x12}" (arg1), "{x13}" (arg2), "{x14}" (arg3), "{x15}" (arg4), "{x16}" (arg5)
            : "memory"
            : "volatile");
        #[cfg(target_arch = "x86_64")]
            asm!("int 0x40"
            : "={rax}" (ret)
            : "{rax}" (id), "{rdi}" (arg0), "{rsi}" (arg1), "{rdx}" (arg2), "{rcx}" (arg3), "{r8}" (arg4), "{r9}" (arg5)
            : "memory"
            : "intel" "volatile");
        #[cfg(target_arch = "aarch64")]
            asm!("svc 0"
            : "={x0}" (ret)
            : "{x8}" (id), "{x0}" (arg0), "{x1}" (arg1), "{x2}" (arg2), "{x3}" (arg3), "{x4}" (arg4), "{x5}" (arg5)
            : "memory"
            : "volatile");
    }
    ret
}

pub fn sys_exit(code: usize) -> ! {
    sys_call(SYS_EXIT, code, 0, 0, 0, 0, 0);
    unreachable!()
}

pub fn sys_write(fd: usize, base: *const u8, len: usize) -> i32 {
    sys_call(SYS_WRITE, fd, base as usize, len, 0, 0, 0)
}

pub fn sys_open(path: &str, flags: usize) -> i32 {
    // UNSAFE: append '\0' to the string
    use core::mem::replace;
    let end = unsafe { &mut *(path.as_ptr().offset(path.len() as isize) as *mut u8) };
    let backup = replace(end, 0);
    let ret = sys_call(SYS_OPEN, path.as_ptr() as usize, flags, 0, 0, 0, 0);
    *end = backup;
    ret
}

pub fn sys_close(fd: usize) -> i32 {
    sys_call(SYS_CLOSE, fd, 0, 0, 0, 0, 0)
}

pub fn sys_dup(fd1: usize, fd2: usize) -> i32 {
    sys_call(SYS_DUP, fd1, fd2, 0, 0, 0, 0)
}

/// Fork the current process. Return the child's PID.
pub fn sys_fork() -> i32 {
    sys_call(SYS_FORK, 0, 0, 0, 0, 0, 0)
}

/// Wait the process exit.
/// Return the PID. Store exit code to `code` if it's not null.
pub fn sys_wait(pid: usize, code: *mut i32) -> i32 {
    sys_call(SYS_WAIT, pid, code as usize, 0, 0, 0, 0)
}

pub fn sys_yield() -> i32 {
    sys_call(SYS_YIELD, 0, 0, 0, 0, 0, 0)
}

/// Kill the process
pub fn sys_kill(pid: usize) -> i32 {
    sys_call(SYS_KILL, pid, 0, 0, 0, 0, 0)
}

/// Get the current process id
pub fn sys_getpid() -> i32 {
    sys_call(SYS_GETPID, 0, 0, 0, 0, 0, 0)
}

pub fn sys_sleep(time: usize) -> i32 {
    sys_call(SYS_SLEEP, time, 0, 0, 0, 0, 0)
}

pub fn sys_get_time() -> i32 {
    sys_call(SYS_GETTIME, 0, 0, 0, 0, 0, 0)
}

pub fn sys_lab6_set_priority(priority: usize) -> i32 {
    sys_call(SYS_LAB6_SET_PRIORITY, priority, 0, 0, 0, 0, 0)
}

pub fn sys_putc(c: char) -> i32 {
    sys_call(SYS_PUTC, c as usize, 0, 0, 0, 0, 0)
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

/* VFS flags */
// TODO: use bitflags
// flags for open: choose one of these
pub const O_RDONLY: usize = 0; // open for reading only
pub const O_WRONLY: usize = 1; // open for writing only
pub const O_RDWR: usize = 2; // open for reading and writing
// then or in any of these:
pub const O_CREAT: usize = 0x00000004; // create file if it does not exist
pub const O_EXCL: usize = 0x00000008; // error if O_CREAT and the file exists
pub const O_TRUNC: usize = 0x00000010; // truncate file upon open
pub const O_APPEND: usize = 0x00000020; // append on each write
