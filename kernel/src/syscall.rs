//! System call

#![allow(unused)]

use arch::interrupt::TrapFrame;
use process::*;
use thread;
use util;
use simple_filesystem::{INode, file::File};
use core::{slice, str};
use alloc::boxed::Box;

/// System call dispatcher
pub fn syscall(id: usize, args: [usize; 6], tf: &TrapFrame) -> i32 {
    match id {
        // file
        100 => sys_open(args[0] as *const u8, args[1]),
        101 => sys_close(args[0]),
        102 => sys_read(args[0], args[1] as *mut u8, args[2]),
        103 => sys_write(args[0], args[1] as *const u8, args[2]),
        030 => sys_putc(args[0] as u8 as char),
//        104 => sys_seek(),
//        110 => sys_fstat(),
//        111 => sys_fsync(),
//        121 => sys_getcwd(),
//        128 => sys_getdirentry(),
//        128 => sys_dup(),

        // process
        001 => sys_exit(args[0]),
        002 => sys_fork(tf),
        003 => sys_wait(args[0], args[1] as *mut i32),
//        004 => sys_exec(),
//        005 => sys_clone(),
        010 => sys_yield(),
        011 => sys_sleep(args[0]),
        012 => sys_kill(args[0]),
        017 => sys_get_time(),
        018 => sys_getpid(),
        255 => sys_lab6_set_priority(args[0]),

        // memory
//        020 => sys_mmap(),
//        021 => sys_munmap(),
//        022 => sys_shmem(),
//        031 => sys_pgdir(),

        _ => {
            error!("unknown syscall id: {:#x?}, args: {:x?}", id, args);
            ::trap::error(tf);
        }
    }
}

fn sys_read(fd: usize, base: *mut u8, len: usize) -> i32 {
    info!("read: fd: {}, base: {:?}, len: {:#x}", fd, base, len);
    let slice = unsafe { slice::from_raw_parts_mut(base, len) };
    match fd {
        0 => unimplemented!(),
        1 | 2 => return -1,
        _ => {
            let mut file = process().files.get_mut(&fd);
            if file.is_none() {
                return -1;
            }
            let file = file.as_mut().unwrap();
            file.read(slice).unwrap();
        }
    }
    0
}

fn sys_write(fd: usize, base: *const u8, len: usize) -> i32 {
    info!("write: fd: {}, base: {:?}, len: {:#x}", fd, base, len);
    let slice = unsafe { slice::from_raw_parts(base, len) };
    match fd {
        0 => return -1,
        1 | 2 => print!("{}", str::from_utf8(slice).unwrap()),
        _ => {
            let mut file = process().files.get_mut(&fd);
            if file.is_none() {
                return -1;
            }
            let file = file.as_mut().unwrap();
            file.write(slice).unwrap();
        }
    }
    0
}

fn sys_open(path: *const u8, flags: usize) -> i32 {
    let path = unsafe { util::from_cstr(path) };
    let flags = VfsFlags::from_ucore_flags(flags);
    info!("open: path: {:?}, flags: {:?}", path, flags);
    match path {
        "stdin:" => return 0,
        "stdout:" => return 1,
        "stderr:" => return 2,
        _ => {}
    }
    let inode = ::fs::ROOT_INODE.lookup(path);
    if inode.is_err() {
        return -1;
    }
    let inode = inode.unwrap();
    let files = &mut process().files;
    let fd = (3..).find(|i| !files.contains_key(i)).unwrap();
    let file = File::new(inode, flags.contains(VfsFlags::READABLE), flags.contains(VfsFlags::WRITABLE));
    files.insert(fd, Box::new(file));
    fd as i32
}

fn sys_close(fd: usize) -> i32 {
    info!("close: fd: {:?}", fd);
    if fd < 3 {
        return 0;
    }
    match process().files.remove(&fd) {
        Some(_) => 0,
        None => -1,
    }
}

/// Fork the current process. Return the child's PID.
fn sys_fork(tf: &TrapFrame) -> i32 {
    let mut context = process().fork(tf);
    let pid = processor().manager().add(context);
    Process::new_fork(pid, thread::current().id());
    info!("fork: {} -> {}", thread::current().id(), pid);
    pid as i32
}

/// Wait the process exit.
/// Return the PID. Store exit code to `code` if it's not null.
fn sys_wait(pid: usize, code: *mut i32) -> i32 {
    loop {
        let wait_procs = match pid {
            0 => Process::get_children(),
            _ => vec![pid],
        };
        if wait_procs.is_empty() {
            return -1;
        }
        for pid in wait_procs {
            match processor().manager().get_status(pid) {
                Some(Status::Exited(exit_code)) => {
                    if !code.is_null() {
                        unsafe { code.write(exit_code as i32); }
                    }
                    processor().manager().remove(pid);
                    Process::do_wait(pid);
                    info!("wait: {} -> {}", thread::current().id(), pid);
                    return 0;
                }
                None => return -1,
                _ => {}
            }
        }
        info!("wait: {} -> {}, sleep", thread::current().id(), pid);
        if pid == 0 {
            Process::wait_child();
        } else {
            processor().manager().wait(thread::current().id(), pid);
            processor().yield_now();
        }
    }
}

fn sys_yield() -> i32 {
    thread::yield_now();
    0
}

/// Kill the process
fn sys_kill(pid: usize) -> i32 {
    info!("kill: {}", pid);
    processor().manager().exit(pid, 0x100);
    if pid == thread::current().id() {
        processor().yield_now();
    }
    0
}

/// Get the current process id
fn sys_getpid() -> i32 {
    thread::current().id() as i32
}

/// Exit the current process
fn sys_exit(exit_code: usize) -> i32 {
    let pid = thread::current().id();
    info!("exit: {}", pid);
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

bitflags! {
    struct VfsFlags: usize {
        // WARNING: different from origin uCore
        const READABLE = 1 << 0;
        const WRITABLE = 1 << 1;
        /// create file if it does not exist
        const CREATE = 1 << 2;
        /// error if O_CREAT and the file exists
        const EXCLUSIVE = 1 << 3;
        /// truncate file upon open
        const TRUNCATE = 1 << 4;
        /// append on each write
        const APPEND = 1 << 5;
    }
}

impl VfsFlags {
    fn from_ucore_flags(f: usize) -> Self {
        assert_ne!(f & 0b11, 0b11);
        Self::from_bits_truncate(f + 1)
    }
}
