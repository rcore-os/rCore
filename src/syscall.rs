use super::*;
use process;
use arch::interrupt::TrapFrame;

pub unsafe fn syscall(tf: &TrapFrame, rsp: &mut usize, is32: bool) -> i32 {
    let id = match is32 {
        false => Syscall::Xv6(tf.scratch.rax),
        true => Syscall::Ucore(tf.scratch.rax),
    };
    let args = match is32 {
        // For ucore x86
        true => [tf.scratch.rdx, tf.scratch.rcx, tf.preserved.rbx, tf.scratch.rdi, tf.scratch.rsi, 0],
        // For xv6 x86_64
        false => [tf.scratch.rdi, tf.scratch.rsi, tf.scratch.rdx, tf.scratch.rcx, tf.scratch.r8, tf.scratch.r9],
    };

    match id {
        Syscall::Xv6(SYS_WRITE) | Syscall::Ucore(UCORE_SYS_WRITE) =>
            io::write(args[0], args[1] as *const u8, args[2]),
        Syscall::Xv6(SYS_OPEN) | Syscall::Ucore(UCORE_SYS_OPEN) =>
            io::open(args[0] as *const u8, args[1]),
        Syscall::Xv6(SYS_CLOSE) | Syscall::Ucore(UCORE_SYS_CLOSE) =>
            io::close(args[0]),
        Syscall::Xv6(SYS_WAIT) | Syscall::Ucore(UCORE_SYS_WAIT) =>
            process::sys_wait(rsp, args[0]),
        Syscall::Xv6(SYS_FORK) | Syscall::Ucore(UCORE_SYS_FORK) =>
            process::sys_fork(tf),
        Syscall::Xv6(SYS_KILL) | Syscall::Ucore(UCORE_SYS_KILL) =>
            process::sys_kill(args[0]),
        Syscall::Xv6(SYS_EXIT) | Syscall::Ucore(UCORE_SYS_EXIT) =>
            process::sys_exit(rsp, args[0]),
        Syscall::Ucore(UCORE_SYS_GETPID) =>
            process::sys_getpid(),
        Syscall::Ucore(UCORE_SYS_PUTC) =>
            {
                print!("{}", args[0] as u8 as char);
                0
            },
        _ => {
            warn!("unknown syscall {:#x?}", id);
            -1
        },
    }
}

#[derive(Debug)]
enum Syscall {
    Xv6(usize),
    Ucore(usize),
}

const SYS_FORK: usize = 1;
const SYS_EXIT: usize = 2;
const SYS_WAIT: usize = 3;
const SYS_PIPE: usize = 4;
const SYS_READ: usize = 5;
const SYS_KILL: usize = 6;
const SYS_EXEC: usize = 7;
const SYS_FSTAT: usize = 8;
const SYS_CHDIR: usize = 9;
const SYS_DUP: usize = 10;
const SYS_GETPID: usize = 11;
const SYS_SBRK: usize = 12;
const SYS_SLEEP: usize = 13;
const SYS_UPTIME: usize = 14;
const SYS_OPEN: usize = 15;
const SYS_WRITE: usize = 16;
const SYS_MKNOD: usize = 17;
const SYS_UNLINK: usize = 18;
const SYS_LINK: usize = 19;
const SYS_MKDIR: usize = 20;
const SYS_CLOSE: usize = 21;
const SYS_CHMOD: usize = 22;

const UCORE_SYS_EXIT: usize = 1;
const UCORE_SYS_FORK: usize = 2;
const UCORE_SYS_WAIT: usize = 3;
const UCORE_SYS_EXEC: usize = 4;
const UCORE_SYS_CLONE: usize = 5;
const UCORE_SYS_YIELD: usize = 10;
const UCORE_SYS_SLEEP: usize = 11;
const UCORE_SYS_KILL: usize = 12;
const UCORE_SYS_GETTIME: usize = 17;
const UCORE_SYS_GETPID: usize = 18;
const UCORE_SYS_MMAP: usize = 20;
const UCORE_SYS_MUNMAP: usize = 21;
const UCORE_SYS_SHMEM: usize = 22;
const UCORE_SYS_PUTC: usize = 30;
const UCORE_SYS_PGDIR: usize = 31;
const UCORE_SYS_OPEN: usize = 100;
const UCORE_SYS_CLOSE: usize = 101;
const UCORE_SYS_READ: usize = 102;
const UCORE_SYS_WRITE: usize = 103;
const UCORE_SYS_SEEK: usize = 104;
const UCORE_SYS_FSTAT: usize = 110;
const UCORE_SYS_FSYNC: usize = 111;
const UCORE_SYS_GETCWD: usize = 121;
const UCORE_SYS_GETDIRENTRY: usize = 128;
const UCORE_SYS_DUP: usize = 130;
