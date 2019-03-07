//! System call

use alloc::{string::String, sync::Arc, vec::Vec};
use core::{slice, str, fmt};

use bitflags::bitflags;
use rcore_memory::VMError;
use rcore_fs::vfs::{FileType, FsError, INode, Metadata};
use spin::{Mutex, MutexGuard};

use crate::arch::interrupt::TrapFrame;
use crate::fs::FileHandle;
use crate::process::*;
use crate::thread;
use crate::util;
use crate::arch::cpu;

use self::fs::*;
use self::mem::*;
use self::proc::*;
use self::time::*;
use self::ctrl::*;
use self::net::*;

mod fs;
mod mem;
mod proc;
mod time;
mod ctrl;
mod net;

/// System call dispatcher
pub fn syscall(id: usize, args: [usize; 6], tf: &mut TrapFrame) -> isize {
    let pid = cpu::id();
    let tid = processor().tid();
    debug!("{}:{} syscall id {} begin", pid, tid, id);
    let ret = match id {
        // file
        000 => sys_read(args[0], args[1] as *mut u8, args[2]),
        001 => sys_write(args[0], args[1] as *const u8, args[2]),
        002 => sys_open(args[0] as *const u8, args[1], args[2]),
        003 => sys_close(args[0]),
        004 => sys_stat(args[0] as *const u8, args[1] as *mut Stat),
        005 => sys_fstat(args[0], args[1] as *mut Stat),
        006 => sys_lstat(args[0] as *const u8, args[1] as *mut Stat),
        007 => sys_poll(args[0] as *mut PollFd, args[1], args[2]),
        008 => sys_lseek(args[0], args[1] as i64, args[2] as u8),
        009 => sys_mmap(args[0], args[1], args[2], args[3], args[4] as i32, args[5]),
        011 => sys_munmap(args[0], args[1]),
        019 => sys_readv(args[0], args[1] as *const IoVec, args[2]),
        020 => sys_writev(args[0], args[1] as *const IoVec, args[2]),
        021 => sys_access(args[0] as *const u8, args[1]),
        022 => sys_pipe(args[0] as *mut u32),
        023 => sys_select(args[0], args[1] as *mut u32, args[2] as *mut u32, args[3] as *mut u32, args[4] as *const TimeVal),
        024 => sys_yield(),
        033 => sys_dup2(args[0], args[1]),
//        034 => sys_pause(),
        035 => sys_sleep(args[0]), // TODO: nanosleep
        039 => sys_getpid(),
        041 => sys_socket(args[0], args[1], args[2]),
        042 => sys_connect(args[0], args[1] as *const SockaddrIn, args[2]),
        043 => sys_accept(args[0], args[1] as *mut SockaddrIn, args[2] as *mut u32),
        044 => sys_sendto(args[0], args[1] as *const u8, args[2], args[3], args[4] as *const SockaddrIn, args[5]),
        045 => sys_recvfrom(args[0], args[1] as *mut u8, args[2], args[3], args[4] as *mut SockaddrIn, args[5] as *mut u32),
//        046 => sys_sendmsg(),
//        047 => sys_recvmsg(),
        048 => sys_shutdown(args[0], args[1]),
        049 => sys_bind(args[0], args[1] as *const SockaddrIn, args[2]),
        050 => sys_listen(args[0], args[1]),
        051 => sys_getsockname(args[0], args[1] as *mut SockaddrIn, args[2] as *mut u32),
        052 => sys_getpeername(args[0], args[1] as *mut SockaddrIn, args[2] as *mut u32),
        054 => sys_setsockopt(args[0], args[1], args[2], args[3] as *const u8, args[4]),
        055 => sys_getsockopt(args[0], args[1], args[2], args[3] as *mut u8, args[4] as *mut u32),
//        056 => sys_clone(),
        057 => sys_fork(tf),
        // use fork for vfork
        058 => sys_fork(tf),
        059 => sys_exec(args[0] as *const u8, args[1] as *const *const u8, args[2] as *const *const u8, tf),
        060 => sys_exit(args[0] as isize),
        061 => sys_wait(args[0], args[1] as *mut i32), // TODO: wait4
        062 => sys_kill(args[0]),
//        072 => sys_fcntl(),
        074 => sys_fsync(args[0]),
        075 => sys_fdatasync(args[0]),
        076 => sys_truncate(args[0] as *const u8, args[1]),
        077 => sys_ftruncate(args[0], args[1]),
        079 => sys_getcwd(args[0] as *mut u8, args[1]),
        080 => sys_chdir(args[0] as *const u8),
        082 => sys_rename(args[0] as *const u8, args[1] as *const u8),
        083 => sys_mkdir(args[0] as *const u8, args[1]),
        086 => sys_link(args[0] as *const u8, args[1] as *const u8),
        087 => sys_unlink(args[0] as *const u8),
        096 => sys_gettimeofday(args[0] as *mut TimeVal, args[1] as *const u8),
//        097 => sys_getrlimit(),
//        098 => sys_getrusage(),
        110 => sys_getppid(),
//        133 => sys_mknod(),
        141 => sys_set_priority(args[0]),
        158 => sys_arch_prctl(args[0] as i32, args[1], tf),
//        160 => sys_setrlimit(),
//        162 => sys_sync(),
//        169 => sys_reboot(),
        186 => sys_gettid(),
        201 => sys_time(args[0] as *mut u64),
        217 => sys_getdents64(args[0], args[1] as *mut LinuxDirent64, args[2]),
//        293 => sys_pipe(),

        // for musl: empty impl
        012 => {
            warn!("sys_brk is unimplemented");
            Ok(0)
        }
        013 => {
            warn!("sys_sigaction is unimplemented");
            Ok(0)
        }
        014 => {
            warn!("sys_sigprocmask is unimplemented");
            Ok(0)
        }
        016 => {
            warn!("sys_ioctl is unimplemented");
            Ok(0)
        }
        037 => {
            warn!("sys_alarm is unimplemented");
            Ok(0)
        }
        072 => {
            warn!("sys_fcntl is unimplemented");
            Ok(0)
        }
        095 => {
            warn!("sys_umask is unimplemented");
            Ok(0o777)
        }
        102 => {
            warn!("sys_getuid is unimplemented");
            Ok(0)
        }
        105 => {
            warn!("sys_setuid is unimplemented");
            Ok(0)
        }
        107 => {
            warn!("sys_geteuid is unimplemented");
            Ok(0)
        }
        108 => {
            warn!("sys_getegid is unimplemented");
            Ok(0)
        }
        112 => {
            warn!("sys_setsid is unimplemented");
            Ok(0)
        }
        131 => {
            warn!("sys_sigaltstack is unimplemented");
            Ok(0)
        }
        218 => {
            warn!("sys_set_tid_address is unimplemented");
            Ok(thread::current().id() as isize)
        }
        228 => {
            warn!("sys_clock_gettime is unimplemented");
            Ok(0)
        }
        231 => {
            warn!("sys_exit_group is unimplemented");
            sys_exit(args[0] as isize);
        }
        _ => {
            error!("unknown syscall id: {:#x?}, args: {:x?}", id, args);
            crate::trap::error(tf);
        }
    };
    debug!("{}:{} syscall id {} ret with {:?}", pid, tid, id, ret);
    match ret {
        Ok(code) => code,
        Err(err) => -(err as isize),
    }
}

pub type SysResult = Result<isize, SysError>;

#[allow(dead_code)]
#[repr(isize)]
#[derive(Debug)]
pub enum SysError {
    EUNDEF = 0,
    EPERM = 1,
    ENOENT = 2,
    ESRCH = 3,
    EINTR = 4,
    EIO = 5,
    ENXIO = 6,
    E2BIG = 7,
    ENOEXEC = 8,
    EBADF = 9,
    ECHILD = 10,
    EAGAIN = 11,
    ENOMEM = 12,
    EACCES = 13,
    EFAULT = 14,
    ENOTBLK = 15,
    EBUSY = 16,
    EEXIST = 17,
    EXDEV = 18,
    ENODEV = 19,
    ENOTDIR = 20,
    EISDIR = 21,
    EINVAL = 22,
    ENFILE = 23,
    EMFILE = 24,
    ENOTTY = 25,
    ETXTBSY = 26,
    EFBIG = 27,
    ENOSPC = 28,
    ESPIPE = 29,
    EROFS = 30,
    EMLINK = 31,
    EPIPE = 32,
    EDOM = 33,
    ERANGE = 34,
    EDEADLK = 35,
    ENAMETOOLONG = 36,
    ENOLCK = 37,
    ENOSYS = 38,
    ENOTEMPTY = 39,
    ENOTSOCK = 80,
    ENOPROTOOPT = 92,
    EPFNOSUPPORT = 96,
    EAFNOSUPPORT = 97,
    ENOBUFS = 105,
    EISCONN = 106,
    ENOTCONN = 107,
    ECONNREFUSED = 111,
}

#[allow(non_snake_case)]
impl fmt::Display for SysError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}",
            match self {
                EPERM => "Operation not permitted",
                ENOENT => "No such file or directory",
                ESRCH => "No such process",
                EINTR => "Interrupted system call",
                EIO => "I/O error",
                ENXIO => "No such device or address",
                E2BIG => "Argument list too long",
                ENOEXEC => "Exec format error",
                EBADF => "Bad file number",
                ECHILD => "No child processes",
                EAGAIN => "Try again",
                ENOMEM => "Out of memory",
                EACCES => "Permission denied",
                EFAULT => "Bad address",
                ENOTBLK => "Block device required",
                EBUSY => "Device or resource busy",
                EEXIST => "File exists",
                EXDEV => "Cross-device link",
                ENODEV => "No such device",
                ENOTDIR => "Not a directory",
                EISDIR => "Is a directory",
                EINVAL => "Invalid argument",
                ENFILE => "File table overflow",
                EMFILE => "Too many open files",
                ENOTTY => "Not a typewriter",
                ETXTBSY => "Text file busy",
                EFBIG => "File too large",
                ENOSPC => "No space left on device",
                ESPIPE => "Illegal seek",
                EROFS => "Read-only file system",
                EMLINK => "Too many links",
                EPIPE => "Broken pipe",
                EDOM => "Math argument out of domain of func",
                ERANGE => "Math result not representable",
                EDEADLK => "Resource deadlock would occur",
                ENAMETOOLONG => "File name too long",
                ENOLCK => "No record locks available",
                ENOSYS => "Function not implemented",
                ENOTEMPTY => "Directory not empty",
                _ => "Unknown error",
            },
        )
    }
}

impl From<VMError> for SysError {
    fn from(_: VMError) -> Self {
        SysError::EFAULT
    }
}
