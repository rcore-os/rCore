//! System call

use alloc::{string::String, sync::Arc, vec::Vec};
use core::{fmt, slice, str};

use bitflags::bitflags;
use rcore_fs::vfs::{FileType, FsError, INode, Metadata};
use rcore_memory::VMError;

use crate::arch::cpu;
use crate::arch::interrupt::TrapFrame;
use crate::arch::syscall::*;
use crate::memory::{copy_from_user, MemorySet};
use crate::process::*;
use crate::sync::{Condvar, MutexGuard, SpinNoIrq};
use crate::thread;
use crate::util;

pub use self::custom::*;
pub use self::fs::*;
pub use self::lkm::*;
pub use self::mem::*;
pub use self::misc::*;
pub use self::net::*;
pub use self::proc::*;
pub use self::time::*;

mod custom;
mod fs;
mod lkm;
mod mem;
mod misc;
mod net;
mod proc;
mod time;

#[cfg(feature = "profile")]
use alloc::collections::BTreeMap;
#[cfg(feature = "profile")]
use spin::Mutex;

#[cfg(feature = "profile")]
lazy_static! {
    static ref SYSCALL_TIMING: Mutex<BTreeMap<usize, i64>> = Mutex::new(BTreeMap::new());
}

/// System call dispatcher
pub fn syscall(id: usize, args: [usize; 6], tf: &mut TrapFrame) -> isize {
    let thread = unsafe { current_thread() };
    let mut syscall = Syscall { thread, tf };
    syscall.syscall(id, args)
}

/// All context needed for syscall
struct Syscall<'a> {
    thread: &'a mut Thread,
    tf: &'a mut TrapFrame,
}

impl Syscall<'_> {
    /// Get current process
    pub fn process(&self) -> MutexGuard<'_, Process, SpinNoIrq> {
        self.thread.proc.lock()
    }

    /// Get current virtual memory
    pub fn vm(&self) -> MutexGuard<'_, MemorySet, SpinNoIrq> {
        self.thread.vm.lock()
    }

    /// System call dispatcher
    // This #[deny(unreachable_patterns)] checks if each match arm is defined
    // See discussion in https://github.com/oscourse-tsinghua/rcore_plus/commit/17e644e54e494835f1a49b34b80c2c4f15ed0dbe.
    #[deny(unreachable_patterns)]
    fn syscall(&mut self, id: usize, args: [usize; 6]) -> isize {
        #[cfg(feature = "profile")]
        let begin_time = unsafe { core::arch::x86_64::_rdtsc() };
        let cid = cpu::id();
        let pid = self.process().pid.clone();
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
            SYS_READ => self.sys_read(args[0], args[1] as *mut u8, args[2]),
            SYS_WRITE => self.sys_write(args[0], args[1] as *const u8, args[2]),
            SYS_OPENAT => self.sys_openat(args[0], args[1] as *const u8, args[2], args[3]),
            SYS_CLOSE => self.sys_close(args[0]),
            SYS_FSTAT => self.sys_fstat(args[0], args[1] as *mut Stat),
            SYS_NEWFSTATAT => {
                self.sys_fstatat(args[0], args[1] as *const u8, args[2] as *mut Stat, args[3])
            }
            SYS_LSEEK => self.sys_lseek(args[0], args[1] as i64, args[2] as u8),
            SYS_IOCTL => self.sys_ioctl(args[0], args[1], args[2], args[3], args[4]),
            SYS_PREAD64 => self.sys_pread(args[0], args[1] as *mut u8, args[2], args[3]),
            SYS_PWRITE64 => self.sys_pwrite(args[0], args[1] as *const u8, args[2], args[3]),
            SYS_READV => self.sys_readv(args[0], args[1] as *const IoVec, args[2]),
            SYS_WRITEV => self.sys_writev(args[0], args[1] as *const IoVec, args[2]),
            SYS_SENDFILE => self.sys_sendfile(args[0], args[1], args[2] as *mut usize, args[3]),
            SYS_FCNTL => {
                info!(
                    "SYS_FCNTL : {} {} {} {}",
                    args[0], args[1], args[2], args[3]
                );
                self.sys_fcntl(args[0], args[1], args[2])
            }
            SYS_FLOCK => self.unimplemented("flock", Ok(0)),
            SYS_FSYNC => self.sys_fsync(args[0]),
            SYS_FDATASYNC => self.sys_fdatasync(args[0]),
            SYS_TRUNCATE => self.sys_truncate(args[0] as *const u8, args[1]),
            SYS_FTRUNCATE => self.sys_ftruncate(args[0], args[1]),
            SYS_GETDENTS64 => self.sys_getdents64(args[0], args[1] as *mut LinuxDirent64, args[2]),
            SYS_GETCWD => self.sys_getcwd(args[0] as *mut u8, args[1]),
            SYS_CHDIR => self.sys_chdir(args[0] as *const u8),
            SYS_RENAMEAT => {
                self.sys_renameat(args[0], args[1] as *const u8, args[2], args[3] as *const u8)
            }
            SYS_MKDIRAT => self.sys_mkdirat(args[0], args[1] as *const u8, args[2]),
            SYS_LINKAT => self.sys_linkat(
                args[0],
                args[1] as *const u8,
                args[2],
                args[3] as *const u8,
                args[4],
            ),
            SYS_UNLINKAT => self.sys_unlinkat(args[0], args[1] as *const u8, args[2]),
            SYS_SYMLINKAT => self.unimplemented("symlinkat", Err(SysError::EACCES)),
            SYS_READLINKAT => {
                self.sys_readlinkat(args[0], args[1] as *const u8, args[2] as *mut u8, args[3])
            }
            SYS_FCHMOD => self.unimplemented("fchmod", Ok(0)),
            SYS_FCHMODAT => self.unimplemented("fchmodat", Ok(0)),
            SYS_FCHOWN => self.unimplemented("fchown", Ok(0)),
            SYS_FCHOWNAT => self.unimplemented("fchownat", Ok(0)),
            SYS_FACCESSAT => self.sys_faccessat(args[0], args[1] as *const u8, args[2], args[3]),
            SYS_DUP3 => self.sys_dup2(args[0], args[1]), // TODO: handle `flags`
            SYS_PIPE2 => self.sys_pipe(args[0] as *mut u32), // TODO: handle `flags`
            SYS_UTIMENSAT => self.unimplemented("utimensat", Ok(0)),
            SYS_COPY_FILE_RANGE => self.sys_copy_file_range(
                args[0],
                args[1] as *mut usize,
                args[2],
                args[3] as *mut usize,
                args[4],
                args[5],
            ),

            // io multiplexing
            SYS_PPOLL => {
                self.sys_ppoll(args[0] as *mut PollFd, args[1], args[2] as *const TimeSpec)
            } // ignore sigmask
            SYS_EPOLL_CREATE1 => self.unimplemented("epoll_create1", Err(SysError::ENOSYS)),

            // file system
            SYS_STATFS => self.unimplemented("statfs", Err(SysError::EACCES)),
            SYS_FSTATFS => self.unimplemented("fstatfs", Err(SysError::EACCES)),
            SYS_SYNC => self.sys_sync(),
            SYS_MOUNT => self.unimplemented("mount", Err(SysError::EACCES)),
            SYS_UMOUNT2 => self.unimplemented("umount2", Err(SysError::EACCES)),

            // memory
            SYS_BRK => self.unimplemented("brk", Err(SysError::ENOMEM)),
            SYS_MMAP => self.sys_mmap(args[0], args[1], args[2], args[3], args[4], args[5]),
            SYS_MPROTECT => self.sys_mprotect(args[0], args[1], args[2]),
            SYS_MUNMAP => self.sys_munmap(args[0], args[1]),
            SYS_MADVISE => self.unimplemented("madvise", Ok(0)),

            // signal
            SYS_RT_SIGACTION => self.unimplemented("sigaction", Ok(0)),
            SYS_RT_SIGPROCMASK => self.unimplemented("sigprocmask", Ok(0)),
            SYS_SIGALTSTACK => self.unimplemented("sigaltstack", Ok(0)),
            SYS_KILL => self.sys_kill(args[0], args[1]),

            // schedule
            SYS_SCHED_YIELD => self.sys_yield(),
            SYS_SCHED_GETAFFINITY => {
                self.sys_sched_getaffinity(args[0], args[1], args[2] as *mut u32)
            }

            // socket
            SYS_SOCKET => self.sys_socket(args[0], args[1], args[2]),
            SYS_CONNECT => self.sys_connect(args[0], args[1] as *const SockAddr, args[2]),
            SYS_ACCEPT => self.sys_accept(args[0], args[1] as *mut SockAddr, args[2] as *mut u32),
            SYS_ACCEPT4 => self.sys_accept(args[0], args[1] as *mut SockAddr, args[2] as *mut u32), // use accept for accept4
            SYS_SENDTO => self.sys_sendto(
                args[0],
                args[1] as *const u8,
                args[2],
                args[3],
                args[4] as *const SockAddr,
                args[5],
            ),
            SYS_RECVFROM => self.sys_recvfrom(
                args[0],
                args[1] as *mut u8,
                args[2],
                args[3],
                args[4] as *mut SockAddr,
                args[5] as *mut u32,
            ),
            //        SYS_SENDMSG => self.sys_sendmsg(),
            SYS_RECVMSG => self.sys_recvmsg(args[0], args[1] as *mut MsgHdr, args[2]),
            SYS_SHUTDOWN => self.sys_shutdown(args[0], args[1]),
            SYS_BIND => self.sys_bind(args[0], args[1] as *const SockAddr, args[2]),
            SYS_LISTEN => self.sys_listen(args[0], args[1]),
            SYS_GETSOCKNAME => {
                self.sys_getsockname(args[0], args[1] as *mut SockAddr, args[2] as *mut u32)
            }
            SYS_GETPEERNAME => {
                self.sys_getpeername(args[0], args[1] as *mut SockAddr, args[2] as *mut u32)
            }
            SYS_SETSOCKOPT => {
                self.sys_setsockopt(args[0], args[1], args[2], args[3] as *const u8, args[4])
            }
            SYS_GETSOCKOPT => self.sys_getsockopt(
                args[0],
                args[1],
                args[2],
                args[3] as *mut u8,
                args[4] as *mut u32,
            ),

            // process
            SYS_CLONE => self.sys_clone(
                args[0],
                args[1],
                args[2] as *mut u32,
                args[3] as *mut u32,
                args[4],
            ),
            SYS_EXECVE => self.sys_exec(
                args[0] as *const u8,
                args[1] as *const *const u8,
                args[2] as *const *const u8,
            ),
            SYS_EXIT => self.sys_exit(args[0] as usize),
            SYS_EXIT_GROUP => self.sys_exit_group(args[0]),
            SYS_WAIT4 => self.sys_wait4(args[0] as isize, args[1] as *mut i32), // TODO: wait4
            SYS_SET_TID_ADDRESS => self.sys_set_tid_address(args[0] as *mut u32),
            SYS_FUTEX => self.sys_futex(
                args[0],
                args[1] as u32,
                args[2] as i32,
                args[3] as *const TimeSpec,
            ),
            SYS_TKILL => self.unimplemented("tkill", Ok(0)),

            // time
            SYS_NANOSLEEP => self.sys_nanosleep(args[0] as *const TimeSpec),
            SYS_SETITIMER => self.unimplemented("setitimer", Ok(0)),
            SYS_GETTIMEOFDAY => {
                self.sys_gettimeofday(args[0] as *mut TimeVal, args[1] as *const u8)
            }
            SYS_CLOCK_GETTIME => self.sys_clock_gettime(args[0], args[1] as *mut TimeSpec),

            // system
            SYS_GETPID => self.sys_getpid(),
            SYS_GETTID => self.sys_gettid(),
            SYS_UNAME => self.sys_uname(args[0] as *mut u8),
            SYS_UMASK => self.unimplemented("umask", Ok(0o777)),
            //        SYS_GETRLIMIT => self.sys_getrlimit(),
            //        SYS_SETRLIMIT => self.sys_setrlimit(),
            SYS_GETRUSAGE => self.sys_getrusage(args[0], args[1] as *mut RUsage),
            SYS_SYSINFO => self.sys_sysinfo(args[0] as *mut SysInfo),
            SYS_TIMES => self.sys_times(args[0] as *mut Tms),
            SYS_GETUID => self.unimplemented("getuid", Ok(0)),
            SYS_GETGID => self.unimplemented("getgid", Ok(0)),
            SYS_SETUID => self.unimplemented("setuid", Ok(0)),
            SYS_GETEUID => self.unimplemented("geteuid", Ok(0)),
            SYS_GETEGID => self.unimplemented("getegid", Ok(0)),
            SYS_SETPGID => self.unimplemented("setpgid", Ok(0)),
            SYS_GETPPID => self.sys_getppid(),
            SYS_SETSID => self.unimplemented("setsid", Ok(0)),
            SYS_GETPGID => self.unimplemented("getpgid", Ok(0)),
            SYS_GETGROUPS => self.unimplemented("getgroups", Ok(0)),
            SYS_SETGROUPS => self.unimplemented("setgroups", Ok(0)),
            SYS_SETPRIORITY => self.sys_set_priority(args[0]),
            SYS_PRCTL => self.unimplemented("prctl", Ok(0)),
            SYS_MEMBARRIER => self.unimplemented("membarrier", Ok(0)),
            SYS_PRLIMIT64 => self.sys_prlimit64(
                args[0],
                args[1],
                args[2] as *const RLimit,
                args[3] as *mut RLimit,
            ),
            SYS_REBOOT => self.sys_reboot(
                args[0] as u32,
                args[1] as u32,
                args[2] as u32,
                args[3] as *const u8,
            ),
            SYS_GETRANDOM => {
                self.sys_getrandom(args[0] as *mut u8, args[1] as usize, args[2] as u32)
            }
            SYS_RT_SIGQUEUEINFO => self.unimplemented("rt_sigqueueinfo", Ok(0)),

            // kernel module
            SYS_INIT_MODULE => {
                self.sys_init_module(args[0] as *const u8, args[1] as usize, args[2] as *const u8)
            }
            SYS_FINIT_MODULE => {
                debug!("[LKM] sys_finit_module is unimplemented");
                Err(SysError::ENOSYS)
            }
            SYS_DELETE_MODULE => self.sys_delete_module(args[0] as *const u8, args[1] as u32),

            // custom
            SYS_MAP_PCI_DEVICE => self.sys_map_pci_device(args[0], args[1]),
            SYS_GET_PADDR => {
                self.sys_get_paddr(args[0] as *const u64, args[1] as *mut u64, args[2])
            }

            _ => {
                let ret = match () {
                    #[cfg(target_arch = "x86_64")]
                    () => self.x86_64_syscall(id, args),
                    #[cfg(target_arch = "mips")]
                    () => self.mips_syscall(id, args),
                    #[cfg(all(not(target_arch = "x86_64"), not(target_arch = "mips")))]
                    () => None,
                };
                if let Some(ret) = ret {
                    ret
                } else {
                    error!("unknown syscall id: {}, args: {:x?}", id, args);
                    crate::trap::error(self.tf);
                }
            }
        };
        if !pid.is_init() {
            // we trust pid 0 process
            info!("=> {:x?}", ret);
        }
        #[cfg(feature = "profile")]
        {
            let end_time = unsafe { core::arch::x86_64::_rdtsc() };
            *SYSCALL_TIMING.lock().entry(id).or_insert(0) += end_time - begin_time;
            if end_time % 1000 == 0 {
                let timing = SYSCALL_TIMING.lock();
                let mut count_vec: Vec<(&usize, &i64)> = timing.iter().collect();
                count_vec.sort_by(|a, b| b.1.cmp(a.1));
                for (id, time) in count_vec.iter().take(5) {
                    warn!("timing {:03} time {:012}", id, time);
                }
            }
        }
        match ret {
            Ok(code) => code as isize,
            Err(err) => -(err as isize),
        }
    }

    fn unimplemented(&self, name: &str, ret: SysResult) -> SysResult {
        warn!("{} is unimplemented", name);
        ret
    }

    #[cfg(target_arch = "mips")]
    fn mips_syscall(&mut self, id: usize, args: [usize; 6]) -> Option<SysResult> {
        let ret = match id {
            SYS_OPEN => self.sys_open(args[0] as *const u8, args[1], args[2]),
            SYS_POLL => self.sys_poll(args[0] as *mut PollFd, args[1], args[2]),
            SYS_DUP2 => self.sys_dup2(args[0], args[1]),
            SYS_FORK => self.sys_fork(),
            SYS_MMAP2 => self.sys_mmap(args[0], args[1], args[2], args[3], args[4], args[5] * 4096),
            SYS_FSTAT64 => self.sys_fstat(args[0], args[1] as *mut Stat),
            SYS_LSTAT64 => self.sys_lstat(args[0] as *const u8, args[1] as *mut Stat),
            SYS_STAT64 => self.sys_stat(args[0] as *const u8, args[1] as *mut Stat),
            SYS_PIPE => {
                let fd_ptr = args[0] as *mut u32;
                match self.sys_pipe(fd_ptr) {
                    Ok(code) => {
                        unsafe {
                            self.tf.v0 = *fd_ptr as usize;
                            self.tf.v1 = *(fd_ptr.add(1)) as usize;
                        }
                        Ok(self.tf.v0)
                    }
                    Err(err) => Err(err),
                }
            }
            SYS_FCNTL64 => self.unimplemented("fcntl64", Ok(0)),
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
    fn x86_64_syscall(&mut self, id: usize, args: [usize; 6]) -> Option<SysResult> {
        let ret = match id {
            SYS_OPEN => self.sys_open(args[0] as *const u8, args[1], args[2]),
            SYS_STAT => self.sys_stat(args[0] as *const u8, args[1] as *mut Stat),
            SYS_LSTAT => self.sys_lstat(args[0] as *const u8, args[1] as *mut Stat),
            SYS_POLL => self.sys_poll(args[0] as *mut PollFd, args[1], args[2]),
            SYS_ACCESS => self.sys_access(args[0] as *const u8, args[1]),
            SYS_PIPE => self.sys_pipe(args[0] as *mut u32),
            SYS_SELECT => self.sys_select(
                args[0],
                args[1] as *mut u32,
                args[2] as *mut u32,
                args[3] as *mut u32,
                args[4] as *const TimeVal,
            ),
            SYS_DUP2 => self.sys_dup2(args[0], args[1]),
            SYS_ALARM => self.unimplemented("alarm", Ok(0)),
            SYS_FORK => self.sys_fork(),
            SYS_VFORK => self.sys_vfork(),
            SYS_RENAME => self.sys_rename(args[0] as *const u8, args[1] as *const u8),
            SYS_MKDIR => self.sys_mkdir(args[0] as *const u8, args[1]),
            SYS_RMDIR => self.sys_rmdir(args[0] as *const u8),
            SYS_LINK => self.sys_link(args[0] as *const u8, args[1] as *const u8),
            SYS_UNLINK => self.sys_unlink(args[0] as *const u8),
            SYS_READLINK => self.sys_readlink(args[0] as *const u8, args[1] as *mut u8, args[2]),
            SYS_CHMOD => self.unimplemented("chmod", Ok(0)),
            SYS_CHOWN => self.unimplemented("chown", Ok(0)),
            SYS_ARCH_PRCTL => self.sys_arch_prctl(args[0] as i32, args[1]),
            SYS_TIME => self.sys_time(args[0] as *mut u64),
            SYS_EPOLL_CREATE => self.unimplemented("epoll_create", Err(SysError::ENOSYS)),
            _ => return None,
        };
        Some(ret)
    }
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
    ELOOP = 40,
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
                ELOOP => "Too many symbolic links encountered",
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
    Condvar::wait_events(&condvars, action)
}

pub fn check_and_clone_cstr(user: *const u8) -> Result<String, SysError> {
    if user.is_null() {
        Ok(String::new())
    } else {
        let mut buffer = Vec::new();
        for i in 0.. {
            let addr = unsafe { user.add(i) };
            let data = copy_from_user(addr).ok_or(SysError::EFAULT)?;
            if data == 0 {
                break;
            }
            buffer.push(data);
        }
        String::from_utf8(buffer).map_err(|_| SysError::EFAULT)
    }
}

pub fn check_and_clone_cstr_array(user: *const *const u8) -> Result<Vec<String>, SysError> {
    if user.is_null() {
        Ok(Vec::new())
    } else {
        let mut buffer = Vec::new();
        for i in 0.. {
            let addr = unsafe { user.add(i) };
            let str_ptr = copy_from_user(addr).ok_or(SysError::EFAULT)?;
            if str_ptr.is_null() {
                break;
            }
            let string = check_and_clone_cstr(str_ptr)?;
            buffer.push(string);
        }
        Ok(buffer)
    }
}
