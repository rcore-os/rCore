use super::*;
use process;
use arch::interrupt::TrapFrame;

pub unsafe fn syscall(tf: &TrapFrame, is32: bool) -> i32 {
    let id = match is32 {
        false => tf.scratch.rax,
        true => match tf.scratch.rax {
            UCORE_SYS_OPEN => SYS_OPEN,
            UCORE_SYS_CLOSE => SYS_CLOSE,
            UCORE_SYS_WRITE => SYS_WRITE,
            UCORE_SYS_READ => SYS_READ,
            _ => 0,
        }
    };
    let args = match is32 {
        // For ucore x86
        true => [tf.scratch.rdx, tf.scratch.rcx, tf.preserved.rbx, tf.scratch.rdi, tf.scratch.rsi, 0],
        // For xv6 x86_64
        false => [tf.scratch.rdi, tf.scratch.rsi, tf.scratch.rdx, tf.scratch.rcx, tf.scratch.r8, tf.scratch.r9],
    };
    debug!("id: {:#x}, args: {:#x?}", id, args);

    match id {
        SYS_FORK => {
            process::fork(tf);
            0
        }
        SYS_WRITE => {
            io::write(args[0], args[1] as *const u8, args[2]);
            0
        }
        _ => {
            debug!("unknown syscall {:#x}", id);
            -1
        }
    }
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
