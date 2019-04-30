//! System call

use alloc::{string::String, sync::Arc, vec::Vec};
use core::{fmt, slice, str};

use bitflags::bitflags;
use rcore_fs::vfs::{FileType, FsError, INode, Metadata};
use rcore_memory::VMError;

use crate::arch::cpu;
use crate::arch::interrupt::TrapFrame;
use crate::arch::syscall::*;
use crate::process::*;
use crate::sync::Condvar;
use crate::thread;
use crate::util;

use self::custom::*;
use self::fs::*;
use self::mem::*;
use self::misc::*;
pub use self::net::*;
use self::proc::*;
use self::time::*;

mod custom;
mod fs;
mod mem;
mod misc;
mod net;
mod proc;
mod time;

/// System call dispatcher
// This #[deny(unreachable_patterns)] checks if each match arm is defined
// See discussion in https://github.com/oscourse-tsinghua/rcore_plus/commit/17e644e54e494835f1a49b34b80c2c4f15ed0dbe.
#[deny(unreachable_patterns)]
pub fn syscall(id: usize, args: [usize; 6], tf: &mut TrapFrame) -> isize {
    let cid = cpu::id();
    let pid = process().pid.clone();
    let tid = processor().tid();
    if !pid.is_init() {
        // we trust pid 0 process
        debug!("{}:{}:{} syscall id {} begin", cid, pid, tid, id);
    }

    // use platform-specific syscal numbers
    // See https://filippo.io/linux-syscall-table/
    // And https://fedora.juszkiewicz.com.pl/syscalls.html.
    let ret = match id {
        // file
        SYS_READ => sys_read(args[0], args[1] as *mut u8, args[2]),
        SYS_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYS_OPENAT => sys_openat(args[0], args[1] as *const u8, args[2], args[3]),
        SYS_CLOSE => sys_close(args[0]),
        SYS_FSTAT => sys_fstat(args[0], args[1] as *mut Stat),
        SYS_NEWFSTATAT => sys_fstatat(args[0], args[1] as *const u8, args[2] as *mut Stat, args[3]),
        SYS_LSEEK => sys_lseek(args[0], args[1] as i64, args[2] as u8),
        SYS_IOCTL => sys_ioctl(args[0], args[1], args[2], args[3], args[4]),
        SYS_PREAD64 => sys_pread(args[0], args[1] as *mut u8, args[2], args[3]),
        SYS_PWRITE64 => sys_pwrite(args[0], args[1] as *const u8, args[2], args[3]),
        SYS_READV => sys_readv(args[0], args[1] as *const IoVec, args[2]),
        SYS_WRITEV => sys_writev(args[0], args[1] as *const IoVec, args[2]),
        SYS_SENDFILE => sys_sendfile(args[0], args[1], args[2] as *mut usize, args[3]),
        SYS_FCNTL => unimplemented("fcntl", Ok(0)),
        SYS_FLOCK => unimplemented("flock", Ok(0)),
        SYS_FSYNC => sys_fsync(args[0]),
        SYS_FDATASYNC => sys_fdatasync(args[0]),
        SYS_TRUNCATE => sys_truncate(args[0] as *const u8, args[1]),
        SYS_FTRUNCATE => sys_ftruncate(args[0], args[1]),
        SYS_GETDENTS64 => sys_getdents64(args[0], args[1] as *mut LinuxDirent64, args[2]),
        SYS_GETCWD => sys_getcwd(args[0] as *mut u8, args[1]),
        SYS_CHDIR => sys_chdir(args[0] as *const u8),
        SYS_RENAMEAT => sys_renameat(args[0], args[1] as *const u8, args[2], args[3] as *const u8),
        SYS_MKDIRAT => sys_mkdirat(args[0], args[1] as *const u8, args[2]),
        SYS_LINKAT => sys_linkat(
            args[0],
            args[1] as *const u8,
            args[2],
            args[3] as *const u8,
            args[4],
        ),
        SYS_UNLINKAT => sys_unlinkat(args[0], args[1] as *const u8, args[2]),
        SYS_SYMLINKAT => unimplemented("symlinkat", Err(SysError::EACCES)),
        SYS_READLINKAT => {
            sys_readlinkat(args[0], args[1] as *const u8, args[2] as *mut u8, args[3])
        }
        SYS_FCHMOD => unimplemented("fchmod", Ok(0)),
        SYS_FCHMODAT => unimplemented("fchmodat", Ok(0)),
        SYS_FCHOWN => unimplemented("fchown", Ok(0)),
        SYS_FCHOWNAT => unimplemented("fchownat", Ok(0)),
        SYS_FACCESSAT => sys_faccessat(args[0], args[1] as *const u8, args[2], args[3]),
        SYS_DUP3 => sys_dup2(args[0], args[1]), // TODO: handle `flags`
        SYS_PIPE2 => sys_pipe(args[0] as *mut u32), // TODO: handle `flags`
        SYS_UTIMENSAT => unimplemented("utimensat", Ok(0)),

        // io multiplexing
        SYS_PPOLL => sys_ppoll(args[0] as *mut PollFd, args[1], args[2] as *const TimeSpec), // ignore sigmask
        SYS_EPOLL_CREATE1 => unimplemented("epoll_create1", Err(SysError::ENOSYS)),

        // file system
        SYS_STATFS => unimplemented("statfs", Err(SysError::EACCES)),
        SYS_FSTATFS => unimplemented("fstatfs", Err(SysError::EACCES)),
        SYS_SYNC => sys_sync(),
        SYS_MOUNT => unimplemented("mount", Err(SysError::EACCES)),
        SYS_UMOUNT2 => unimplemented("umount2", Err(SysError::EACCES)),

        // memory
        SYS_BRK => unimplemented("brk", Err(SysError::ENOMEM)),
        SYS_MMAP => sys_mmap(args[0], args[1], args[2], args[3], args[4], args[5]),
        SYS_MPROTECT => sys_mprotect(args[0], args[1], args[2]),
        SYS_MUNMAP => sys_munmap(args[0], args[1]),
        SYS_MADVISE => unimplemented("madvise", Ok(0)),

        // signal
        SYS_RT_SIGACTION => unimplemented("sigaction", Ok(0)),
        SYS_RT_SIGPROCMASK => unimplemented("sigprocmask", Ok(0)),
        SYS_SIGALTSTACK => unimplemented("sigaltstack", Ok(0)),
        SYS_KILL => sys_kill(args[0], args[1]),

        // schedule
        SYS_SCHED_YIELD => sys_yield(),
        SYS_SCHED_GETAFFINITY => sys_sched_getaffinity(args[0], args[1], args[2] as *mut u32),

        // socket
        SYS_SOCKET => sys_socket(args[0], args[1], args[2]),
        SYS_CONNECT => sys_connect(args[0], args[1] as *const SockAddr, args[2]),
        SYS_ACCEPT => sys_accept(args[0], args[1] as *mut SockAddr, args[2] as *mut u32),
        SYS_ACCEPT4 => sys_accept(args[0], args[1] as *mut SockAddr, args[2] as *mut u32), // use accept for accept4
        SYS_SENDTO => sys_sendto(
            args[0],
            args[1] as *const u8,
            args[2],
            args[3],
            args[4] as *const SockAddr,
            args[5],
        ),
        SYS_RECVFROM => sys_recvfrom(
            args[0],
            args[1] as *mut u8,
            args[2],
            args[3],
            args[4] as *mut SockAddr,
            args[5] as *mut u32,
        ),
        //        SYS_SENDMSG => sys_sendmsg(),
        SYS_RECVMSG => sys_recvmsg(args[0], args[1] as *mut MsgHdr, args[2]),
        SYS_SHUTDOWN => sys_shutdown(args[0], args[1]),
        SYS_BIND => sys_bind(args[0], args[1] as *const SockAddr, args[2]),
        SYS_LISTEN => sys_listen(args[0], args[1]),
        SYS_GETSOCKNAME => sys_getsockname(args[0], args[1] as *mut SockAddr, args[2] as *mut u32),
        SYS_GETPEERNAME => sys_getpeername(args[0], args[1] as *mut SockAddr, args[2] as *mut u32),
        SYS_SETSOCKOPT => sys_setsockopt(args[0], args[1], args[2], args[3] as *const u8, args[4]),
        SYS_GETSOCKOPT => sys_getsockopt(
            args[0],
            args[1],
            args[2],
            args[3] as *mut u8,
            args[4] as *mut u32,
        ),

        // process
        SYS_CLONE => sys_clone(
            args[0],
            args[1],
            args[2] as *mut u32,
            args[3] as *mut u32,
            args[4],
            tf,
        ),
        SYS_EXECVE => sys_exec(
            args[0] as *const u8,
            args[1] as *const *const u8,
            args[2] as *const *const u8,
            tf,
        ),
        SYS_EXIT => sys_exit(args[0] as usize),
        SYS_EXIT_GROUP => sys_exit_group(args[0]),
        SYS_WAIT4 => sys_wait4(args[0] as isize, args[1] as *mut i32), // TODO: wait4
        SYS_SET_TID_ADDRESS => unimplemented("set_tid_address", Ok(thread::current().id())),
        SYS_FUTEX => sys_futex(
            args[0],
            args[1] as u32,
            args[2] as i32,
            args[3] as *const TimeSpec,
        ),

        // time
        SYS_NANOSLEEP => sys_nanosleep(args[0] as *const TimeSpec),
        SYS_SETITIMER => unimplemented("setitimer", Ok(0)),
        SYS_GETTIMEOFDAY => sys_gettimeofday(args[0] as *mut TimeVal, args[1] as *const u8),
        SYS_CLOCK_GETTIME => sys_clock_gettime(args[0], args[1] as *mut TimeSpec),

        // system
        SYS_GETPID => sys_getpid(),
        SYS_GETTID => sys_gettid(),
        SYS_UNAME => sys_uname(args[0] as *mut u8),
        SYS_UMASK => unimplemented("umask", Ok(0o777)),
        //        SYS_GETRLIMIT => sys_getrlimit(),
        //        SYS_SETRLIMIT => sys_setrlimit(),
        SYS_GETRUSAGE => sys_getrusage(args[0], args[1] as *mut RUsage),
        SYS_SYSINFO => sys_sysinfo(args[0] as *mut SysInfo),
        SYS_TIMES => sys_times(args[0] as *mut Tms),
        SYS_GETUID => unimplemented("getuid", Ok(0)),
        SYS_GETGID => unimplemented("getgid", Ok(0)),
        SYS_SETUID => unimplemented("setuid", Ok(0)),
        SYS_GETEUID => unimplemented("geteuid", Ok(0)),
        SYS_GETEGID => unimplemented("getegid", Ok(0)),
        SYS_SETPGID => unimplemented("setpgid", Ok(0)),
        SYS_GETPPID => sys_getppid(),
        SYS_SETSID => unimplemented("setsid", Ok(0)),
        SYS_GETPGID => unimplemented("getpgid", Ok(0)),
        SYS_GETGROUPS => unimplemented("getgroups", Ok(0)),
        SYS_SETGROUPS => unimplemented("setgroups", Ok(0)),
        SYS_SETPRIORITY => sys_set_priority(args[0]),
        SYS_PRCTL => unimplemented("prctl", Ok(0)),
        SYS_PRLIMIT64 => sys_prlimit64(
            args[0],
            args[1],
            args[2] as *const RLimit,
            args[3] as *mut RLimit,
        ),
        SYS_REBOOT => sys_reboot(
            args[0] as u32,
            args[1] as u32,
            args[2] as u32,
            args[3] as *const u8,
        ),

        // custom
        SYS_MAP_PCI_DEVICE => sys_map_pci_device(args[0], args[1]),
        SYS_GET_PADDR => sys_get_paddr(args[0] as *const u64, args[1] as *mut u64, args[2]),
        //SYS_GETRANDOM => unimplemented("getrandom", Err(SysError::EINVAL)),
        SYS_GETRANDOM => sys_getrandom(args[0] as *mut u8, args[1] as usize, args[2] as u32),
        SYS_TKILL => unimplemented("tkill", Ok(0)),
        _ => {
            let ret = match () {
                #[cfg(target_arch = "x86_64")]
                () => x86_64_syscall(id, args, tf),
                #[cfg(target_arch = "mips")]
                () => mips_syscall(id, args, tf),
                #[cfg(all(not(target_arch = "x86_64"), not(target_arch = "mips")))]
                () => None,
            };
            if let Some(ret) = ret {
                ret
            } else {
                error!("unknown syscall id: {}, args: {:x?}", id, args);
                crate::trap::error(tf);
            }
        }
    };
    if !pid.is_init() {
        // we trust pid 0 process
        info!("=> {:x?}", ret);
    }
    match ret {
        Ok(code) => code as isize,
        Err(err) => -(err as isize),
    }
}

fn unimplemented(name: &str, ret: SysResult) -> SysResult {
    warn!("{} is unimplemented", name);
    ret
}

#[cfg(target_arch = "mips")]
fn mips_syscall(id: usize, args: [usize; 6], tf: &mut TrapFrame) -> Option<SysResult> {
    let ret = match id {
        SYS_OPEN => sys_open(args[0] as *const u8, args[1], args[2]),
        SYS_POLL => sys_poll(args[0] as *mut PollFd, args[1], args[2]),
        SYS_DUP2 => sys_dup2(args[0], args[1]),
        SYS_FORK => sys_fork(tf),
        SYS_MMAP2 => sys_mmap(args[0], args[1], args[2], args[3], args[4], args[5] * 4096),
        SYS_FSTAT64 => sys_fstat(args[0], args[1] as *mut Stat),
        SYS_LSTAT64 => sys_lstat(args[0] as *const u8, args[1] as *mut Stat),
        SYS_STAT64 => sys_stat(args[0] as *const u8, args[1] as *mut Stat),
        SYS_PIPE => {
            let fd_ptr = args[0] as *mut u32;
            match sys_pipe(fd_ptr) {
                Ok(code) => {
                    unsafe {
                        tf.v0 = *fd_ptr as usize;
                        tf.v1 = *(fd_ptr.add(1)) as usize;
                    }
                    Ok(tf.v0)
                }
                Err(err) => Err(err),
            }
        }
        SYS_FCNTL64 => unimplemented("fcntl64", Ok(0)),
        SYS_SET_THREAD_AREA => {
            info!("set_thread_area: tls: 0x{:x}", args[0]);
            extern "C" {
                fn _cur_tls();
            }

            unsafe {
                asm!("mtc0 $0, $$4, 2": :"r"(args[0]));
                *(_cur_tls as *mut usize) = args[0];
            }
            Ok(0)
        }
        _ => return None,
    };
    Some(ret)
}

#[cfg(target_arch = "x86_64")]
fn x86_64_syscall(id: usize, args: [usize; 6], tf: &mut TrapFrame) -> Option<SysResult> {
    let ret = match id {
        SYS_OPEN => sys_open(args[0] as *const u8, args[1], args[2]),
        SYS_STAT => sys_stat(args[0] as *const u8, args[1] as *mut Stat),
        SYS_LSTAT => sys_lstat(args[0] as *const u8, args[1] as *mut Stat),
        SYS_POLL => sys_poll(args[0] as *mut PollFd, args[1], args[2]),
        SYS_ACCESS => sys_access(args[0] as *const u8, args[1]),
        SYS_PIPE => sys_pipe(args[0] as *mut u32),
        SYS_SELECT => sys_select(
            args[0],
            args[1] as *mut u32,
            args[2] as *mut u32,
            args[3] as *mut u32,
            args[4] as *const TimeVal,
        ),
        SYS_DUP2 => sys_dup2(args[0], args[1]),
        SYS_ALARM => unimplemented("alarm", Ok(0)),
        SYS_FORK => sys_fork(tf),
        SYS_VFORK => sys_vfork(tf),
        SYS_RENAME => sys_rename(args[0] as *const u8, args[1] as *const u8),
        SYS_MKDIR => sys_mkdir(args[0] as *const u8, args[1]),
        SYS_RMDIR => sys_rmdir(args[0] as *const u8),
        SYS_LINK => sys_link(args[0] as *const u8, args[1] as *const u8),
        SYS_UNLINK => sys_unlink(args[0] as *const u8),
        SYS_READLINK => sys_readlink(args[0] as *const u8, args[1] as *mut u8, args[2]),
        SYS_CHMOD => unimplemented("chmod", Ok(0)),
        SYS_CHOWN => unimplemented("chown", Ok(0)),
        SYS_ARCH_PRCTL => sys_arch_prctl(args[0] as i32, args[1], tf),
        SYS_TIME => sys_time(args[0] as *mut u64),
        SYS_EPOLL_CREATE => unimplemented("epoll_create", Err(SysError::ENOSYS)),
        _ => return None,
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
        write!(
            f,
            "{}",
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

pub fn spin_and_wait<T>(condvars: &[&Condvar], mut action: impl FnMut() -> Option<T>) -> T {
    for _i in 0..SPIN_WAIT_TIMES {
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
