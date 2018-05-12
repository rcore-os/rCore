use super::*;
use process;
use arch::interrupt::TrapFrame;

pub unsafe fn syscall(tf: &TrapFrame) -> i32 {
    let id = tf.scratch.rax;
    // For ucore:
//    let args = [tf.scratch.rdx, tf.scratch.rcx, tf.preserved.rbx, tf.scratch.rdi, tf.scratch.rsi];
    // For xv6 x86_64
    let args = [tf.scratch.rdi, tf.scratch.rsi, tf.scratch.rdx, tf.scratch.rcx, tf.scratch.r8, tf.scratch.r9];

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
            debug!("unknown syscall {}", id);
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
