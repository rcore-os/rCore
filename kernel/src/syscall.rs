//! 系统调用解析执行模块

#![allow(unused)]

use arch::interrupt::TrapFrame;
use process::*;
use thread;
use util;

/// 系统调用入口点
///
/// 当发生系统调用中断时，中断服务例程将控制权转移到这里。
pub fn syscall(id: usize, args: [usize; 6], tf: &TrapFrame) -> i32 {
    match id {
        SYS_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYS_OPEN => sys_open(args[0] as *const u8, args[1]),
        SYS_CLOSE => sys_close(args[0]),
        SYS_WAIT => sys_wait(args[0], args[1] as *mut i32),
        SYS_FORK => sys_fork(tf),
        SYS_KILL => sys_kill(args[0]),
        SYS_EXIT => sys_exit(args[0]),
        SYS_YIELD => sys_yield(),
        SYS_GETPID => sys_getpid(),
        SYS_SLEEP => sys_sleep(args[0]),
        SYS_GETTIME => sys_get_time(),
        SYS_LAB6_SET_PRIORITY => sys_lab6_set_priority(args[0]),
        SYS_PUTC => sys_putc(args[0] as u8 as char),
        _ => {
            error!("unknown syscall id: {:#x?}, args: {:x?}", id, args);
            ::trap::error(tf);
        }
    }
}

fn sys_write(fd: usize, base: *const u8, len: usize) -> i32 {
    info!("write: fd: {}, base: {:?}, len: {:#x}", fd, base, len);
    use core::slice;
    use core::str;
    let slice = unsafe { slice::from_raw_parts(base, len) };
    print!("{}", str::from_utf8(slice).unwrap());
    0
}

fn sys_open(path: *const u8, flags: usize) -> i32 {
    let path = unsafe { util::from_cstr(path) };
    info!("open: path: {:?}, flags: {:?}", path, flags);
    match path {
        "stdin:" => 0,
        "stdout:" => 1,
        _ => -1,
    }
}

fn sys_close(fd: usize) -> i32 {
    info!("close: fd: {:?}", fd);
    0
}

/// Fork the current process. Return the child's PID.
fn sys_fork(tf: &TrapFrame) -> i32 {
    use core::mem::transmute;
    let (context, _): (&ContextImpl, *const ()) = unsafe { transmute(processor().context()) };
    let pid = processor().manager().add(context.fork(tf));
    info!("fork: {} -> {}", thread::current().id(), pid);
    pid as i32
}

/// Wait the process exit.
/// Return the PID. Store exit code to `code` if it's not null.
fn sys_wait(pid: usize, code: *mut i32) -> i32 {
    assert_ne!(pid, 0, "wait for 0 is not supported yet");
    loop {
        match processor().manager().get_status(pid) {
            Some(Status::Exited(exit_code)) => {
                if !code.is_null() {
                    unsafe { code.write(exit_code as i32); }
                }
                processor().manager().remove(pid);
                return 0;
            }
            None => return -1,
            _ => {}
        }
        processor().manager().wait(thread::current().id(), pid);
        processor().yield_now();
    }
}

fn sys_yield() -> i32 {
    thread::yield_now();
    0
}

/// Kill the process
fn sys_kill(pid: usize) -> i32 {
    processor().manager().exit(pid, 0x100);
    0
}

/// Get the current process id
fn sys_getpid() -> i32 {
    thread::current().id() as i32
}

/// Exit the current process
fn sys_exit(exit_code: usize) -> i32 {
    let pid = thread::current().id();
    processor().manager().exit(pid, exit_code);
    processor().yield_now();
    unreachable!();
}

fn sys_sleep(time: usize) -> i32 {
    use core::time::Duration;
    thread::sleep(Duration::from_millis(time as u64 * 10));
    0
}

fn sys_get_time() -> i32 {
    unsafe { ::trap::TICK as i32 }
}

fn sys_lab6_set_priority(priority: usize) -> i32 {
    let pid = thread::current().id();
    processor().manager().set_priority(pid, priority as u8);
    0
}

fn sys_putc(c: char) -> i32 {
    print!("{}", c);
    0
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
