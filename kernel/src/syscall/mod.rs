//! System call

use alloc::{string::String, sync::Arc, vec::Vec};
use core::{slice, str, fmt};

use bitflags::bitflags;
use rcore_memory::VMError;
use rcore_fs::vfs::{FileType, FsError, INode, Metadata};

use crate::arch::interrupt::TrapFrame;
use crate::sync::Condvar;
use crate::process::*;
use crate::thread;
use crate::util;
use crate::arch::cpu;
use crate::arch::syscall;

use self::fs::*;
use self::mem::*;
use self::proc::*;
use self::time::*;
use self::net::*;
use self::misc::*;
use self::custom::*;

mod fs;
mod mem;
mod proc;
mod time;
mod net;
mod misc;
mod custom;

/// System call dispatcher
pub fn syscall(id: usize, args: [usize; 6], tf: &mut TrapFrame) -> isize {
    let cid = cpu::id();
    let pid = {
        process().pid.clone()
    };
    let tid = processor().tid();
    if !pid.is_init() {
        // we trust pid 0 process
        debug!("{}:{}:{} syscall id {} begin", cid, pid, tid, id);
    }

    // use syscall numbers in Linux x86_64
    // See https://filippo.io/linux-syscall-table/
    // And https://fedora.juszkiewicz.com.pl/syscalls.html.
    let ret = match id {
        // file
        syscall::SYS_READ => sys_read(args[0], args[1] as *mut u8, args[2]),
        syscall::SYS_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        syscall::SYS_CLOSE => sys_close(args[0]),
        syscall::SYS_FSTAT => sys_fstat(args[0], args[1] as *mut Stat),
        syscall::SYS_LSEEK => sys_lseek(args[0], args[1] as i64, args[2] as u8),
        syscall::SYS_MMAP => sys_mmap(args[0], args[1], args[2], args[3], args[4] as i32, args[5]),
        syscall::SYS_MPROTECT => sys_mprotect(args[0], args[1], args[2]),
        syscall::SYS_MUNMAP => sys_munmap(args[0], args[1]),
        syscall::SYS_PREAD64 => sys_pread(args[0], args[1] as *mut u8, args[2], args[3]),
        syscall::SYS_PWRITE64 => sys_pwrite(args[0], args[1] as *const u8, args[2], args[3]),
        syscall::SYS_READV => sys_readv(args[0], args[1] as *const IoVec, args[2]),
        syscall::SYS_WRITEV => sys_writev(args[0], args[1] as *const IoVec, args[2]),
        syscall::SYS_SCHED_YIELD => sys_yield(),
        syscall::SYS_NANOSLEEP => sys_nanosleep(args[0] as *const TimeSpec),
        syscall::SYS_GETPID => sys_getpid(),
        syscall::SYS_SOCKET => sys_socket(args[0], args[1], args[2]),
        syscall::SYS_CONNECT => sys_connect(args[0], args[1] as *const SockAddr, args[2]),
        syscall::SYS_ACCEPT => sys_accept(args[0], args[1] as *mut SockAddr, args[2] as *mut u32),
        syscall::SYS_SENDTO => sys_sendto(args[0], args[1] as *const u8, args[2], args[3], args[4] as *const SockAddr, args[5]),
        syscall::SYS_RECVFROM => sys_recvfrom(args[0], args[1] as *mut u8, args[2], args[3], args[4] as *mut SockAddr, args[5] as *mut u32),
//        syscall::SYS_SENDMSG => sys_sendmsg(),
//        syscall::SYS_RECVMSG => sys_recvmsg(),
        syscall::SYS_SHUTDOWN => sys_shutdown(args[0], args[1]),
        syscall::SYS_BIND => sys_bind(args[0], args[1] as *const SockAddr, args[2]),
        syscall::SYS_LISTEN => sys_listen(args[0], args[1]),
        syscall::SYS_GETSOCKNAME => sys_getsockname(args[0], args[1] as *mut SockAddr, args[2] as *mut u32),
        syscall::SYS_GETPEERNAME => sys_getpeername(args[0], args[1] as *mut SockAddr, args[2] as *mut u32),
        syscall::SYS_SETSOCKOPT => sys_setsockopt(args[0], args[1], args[2], args[3] as *const u8, args[4]),
        syscall::SYS_GETSOCKOPT => sys_getsockopt(args[0], args[1], args[2], args[3] as *mut u8, args[4] as *mut u32),
        syscall::SYS_CLONE => sys_clone(args[0], args[1], args[2] as *mut u32, args[3] as *mut u32, args[4], tf),
        syscall::SYS_EXECVE => sys_exec(args[0] as *const u8, args[1] as *const *const u8, args[2] as *const *const u8, tf),
        syscall::SYS_EXIT => sys_exit(args[0] as usize),
        syscall::SYS_WAIT4 => sys_wait4(args[0] as isize, args[1] as *mut i32), // TODO: wait4
        syscall::SYS_KILL => sys_kill(args[0], args[1]),
        syscall::SYS_UNAME => sys_uname(args[0] as *mut u8),
        syscall::SYS_FSYNC => sys_fsync(args[0]),
        syscall::SYS_FDATASYNC => sys_fdatasync(args[0]),
        syscall::SYS_TRUNCATE => sys_truncate(args[0] as *const u8, args[1]),
        syscall::SYS_FTRUNCATE => sys_ftruncate(args[0], args[1]),
        syscall::SYS_GETCWD => sys_getcwd(args[0] as *mut u8, args[1]),
        syscall::SYS_CHDIR => sys_chdir(args[0] as *const u8),
        syscall::SYS_GETTIMEOFDAY => sys_gettimeofday(args[0] as *mut TimeVal, args[1] as *const u8),
//        syscall::SYS_GETRLIMIT => sys_getrlimit(),
        syscall::SYS_GETRUSAGE => sys_getrusage(args[0], args[1] as *mut RUsage),
        syscall::SYS_SYSINFO => sys_sysinfo(args[0] as *mut SysInfo),
        syscall::SYS_GETPPID => sys_getppid(),
        syscall::SYS_SETPRIORITY => sys_set_priority(args[0]),
//        syscall::SYS_SETRLIMIT => sys_setrlimit(),
//        syscall::SYS_SYNC => sys_sync(),
        syscall::SYS_REBOOT => sys_reboot(args[0] as u32, args[1] as u32, args[2] as u32, args[3] as *const u8),
        syscall::SYS_GETTID => sys_gettid(),
        syscall::SYS_FUTEX => sys_futex(args[0], args[1] as u32, args[2] as i32, args[3] as *const TimeSpec),
        syscall::SYS_SCHED_GETAFFINITY => sys_sched_getaffinity(args[0], args[1], args[2] as *mut u32),
        syscall::SYS_GETDENTS64 => sys_getdents64(args[0], args[1] as *mut LinuxDirent64, args[2]),
        syscall::SYS_CLOCK_GETTIME => sys_clock_gettime(args[0], args[1] as *mut TimeSpec),
        syscall::SYS_EXIT_GROUP => sys_exit_group(args[0]),
        syscall::SYS_OPENAT => sys_open(args[1] as *const u8, args[2], args[3]), // TODO: handle `dfd`
        syscall::SYS_MKDIRAT => sys_mkdir(args[1] as *const u8, args[2]), // TODO: handle `dfd`
//        syscall::SYS_MKNODAT => sys_mknod(),
        syscall::SYS_NEWFSTATAT => sys_stat(args[1] as *const u8, args[2] as *mut Stat), // TODO: handle `dfd`, `flag`
        syscall::SYS_UNLINKAT => sys_unlink(args[1] as *const u8), // TODO: handle `dfd`, `flag`
        syscall::SYS_RENAMEAT => sys_rename(args[1] as *const u8, args[3] as *const u8), // TODO: handle `olddfd`, `newdfd`
        syscall::SYS_LINKAT => sys_link(args[1] as *const u8, args[3] as *const u8), // TODO: handle `olddfd`, `newdfd`, `flags`
        syscall::SYS_FACCESSAT => sys_access(args[1] as *const u8, args[2]), // TODO: handle `dfd`
        syscall::SYS_ACCEPT4 => sys_accept(args[0], args[1] as *mut SockAddr, args[2] as *mut u32), // use accept for accept4
        syscall::SYS_DUP3 => sys_dup2(args[0], args[1]), // TODO: handle `flags`
        syscall::SYS_PIPE2 => sys_pipe(args[0] as *mut u32), // TODO: handle `flags`
        // custom temporary syscall
        syscall::SYS_MAP_PCI_DEVICE => sys_map_pci_device(args[0], args[1]),
        syscall::SYS_GET_PADDR => sys_get_paddr(args[0] as *const u64, args[1] as *mut u64, args[2]),

        // for musl: empty impl
        syscall::SYS_BRK => {
            warn!("sys_brk is unimplemented");
            Ok(0)
        }
        syscall::SYS_RT_SIGACTION => {
            warn!("sys_sigaction is unimplemented");
            Ok(0)
        }
        syscall::SYS_RT_SIGPROCMASK => {
            warn!("sys_sigprocmask is unimplemented");
            Ok(0)
        }
        syscall::SYS_IOCTL => {
            warn!("sys_ioctl is unimplemented");
            Ok(0)
        }
        syscall::SYS_MADVISE => {
            warn!("sys_madvise is unimplemented");
            Ok(0)
        }
        syscall::SYS_SETITIMER => {
            warn!("sys_setitimer is unimplemented");
            Ok(0)
        }
        syscall::SYS_FCNTL => {
            warn!("sys_fcntl is unimplemented");
            Ok(0)
        }
        syscall::SYS_UMASK => {
            warn!("sys_umask is unimplemented");
            Ok(0o777)
        }
        syscall::SYS_GETUID => {
            warn!("sys_getuid is unimplemented");
            Ok(0)
        }
        syscall::SYS_GETGID => {
            warn!("sys_getgid is unimplemented");
            Ok(0)
        }
        syscall::SYS_SETUID => {
            warn!("sys_setuid is unimplemented");
            Ok(0)
        }
        syscall::SYS_GETEUID => {
            warn!("sys_geteuid is unimplemented");
            Ok(0)
        }
        syscall::SYS_GETEGID => {
            warn!("sys_getegid is unimplemented");
            Ok(0)
        }
        syscall::SYS_SETSID => {
            warn!("sys_setsid is unimplemented");
            Ok(0)
        }
        syscall::SYS_SIGALTSTACK => {
            warn!("sys_sigaltstack is unimplemented");
            Ok(0)
        }
        syscall::SYS_SYNC => {
            warn!("sys_sync is unimplemented");
            Ok(0)
        }
        syscall::SYS_SET_TID_ADDRESS => {
            warn!("sys_set_tid_address is unimplemented");
            Ok(thread::current().id())
        }
        syscall::SYS_UTIMENSAT => {
            warn!("sys_utimensat is unimplemented");
            Ok(0)
        }
        syscall::SYS_EPOLL_CREATE1 => {
            warn!("sys_epoll_create1 is unimplemented");
            Err(SysError::ENOSYS)
        }
        syscall::SYS_PRLIMIT64 => {
            warn!("sys_prlimit64 is unimplemented");
            Ok(0)
        }
        _ => {
            #[cfg(target_arch = "x86_64")]
            let x86_64_ret = x86_64_syscall(id, args, tf);
            #[cfg(not(target_arch = "x86_64"))]
            let x86_64_ret = None;
            if let Some(ret) = x86_64_ret {
                ret
            } else {
                error!("unknown syscall id: {}, args: {:x?}", id, args);
                crate::trap::error(tf);
            }
        }
    };
    if !pid.is_init() {
        // we trust pid 0 process
        debug!("{}:{}:{} syscall id {} ret with {:x?}", cid, pid, tid, id, ret);
    }
    match ret {
        Ok(code) => code as isize,
        Err(err) => -(err as isize),
    }
}

#[cfg(target_arch = "x86_64")]
fn x86_64_syscall(id: usize, args: [usize; 6], tf: &mut TrapFrame) -> Option<SysResult> {
    let ret = match id {
        syscall::SYS_OPEN => sys_open(args[0] as *const u8, args[1], args[2]),
        syscall::SYS_STAT => sys_stat(args[0] as *const u8, args[1] as *mut Stat),
        syscall::SYS_LSTAT => sys_lstat(args[0] as *const u8, args[1] as *mut Stat),
        syscall::SYS_POLL => sys_poll(args[0] as *mut PollFd, args[1], args[2]),
        syscall::SYS_ACCESS => sys_access(args[0] as *const u8, args[1]),
        syscall::SYS_PIPE => sys_pipe(args[0] as *mut u32),
        syscall::SYS_SELECT => sys_select(args[0], args[1] as *mut u32, args[2] as *mut u32, args[3] as *mut u32, args[4] as *const TimeVal),
        syscall::SYS_DUP2 => sys_dup2(args[0], args[1]),
//        syscall::SYS_PAUSE => sys_pause(),
        SYS_FORK => sys_fork(tf),
        // use fork for vfork
        syscall::SYS_VFORK => sys_fork(tf),
        syscall::SYS_RENAME => sys_rename(args[0] as *const u8, args[1] as *const u8),
        syscall::SYS_MKDIR => sys_mkdir(args[0] as *const u8, args[1]),
        syscall::SYS_LINK => sys_link(args[0] as *const u8, args[1] as *const u8),
        syscall::SYS_UNLINK => sys_unlink(args[0] as *const u8),
        syscall::SYS_ARCH_PRCTL => sys_arch_prctl(args[0] as i32, args[1], tf),
        syscall::SYS_TIME => sys_time(args[0] as *mut u64),
        syscall::SYS_ALARM => {
            warn!("sys_alarm is unimplemented");
            Ok(0)
        }
        syscall::SYS_READLINK => {
            warn!("sys_readlink is unimplemented");
            Err(SysError::ENOENT)
        }
        syscall::SYS_CHOWN => {
            warn!("sys_chown is unimplemented");
            Ok(0)
        }
        syscall::SYS_EPOLL_CREATE => {
            warn!("sys_epoll_create is unimplemented");
            Err(SysError::ENOSYS)
        }
        _ => {
            return None;
        }
    };
    Some(ret)
}

pub type SysResult = Result<usize, SysError>;

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
        use self::SysError::*;
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
                ENOTSOCK => "Socket operation on non-socket",
                ENOPROTOOPT => "Protocol not available",
                EPFNOSUPPORT => "Protocol family not supported",
                EAFNOSUPPORT => "Address family not supported by protocol",
                ENOBUFS => "No buffer space available",
                EISCONN => "Transport endpoint is already connected",
                ENOTCONN => "Transport endpoint is not connected",
                ECONNREFUSED => "Connection refused",
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


const SPIN_WAIT_TIMES: usize = 100;

pub fn spin_and_wait(condvars: &[&Condvar], mut action: impl FnMut() -> Option<SysResult>) -> SysResult {
    for i in 0..SPIN_WAIT_TIMES {
        if let Some(result) = action() {
            return result;
        }
    }
    loop {
        if let Some(result) = action() {
            return result;
        }
        Condvar::wait_any(&condvars);
    }
}

