//! System call

use arch::interrupt::TrapFrame;
use process::*;
use thread;
use util;
use simple_filesystem::{INode, file::File, FileInfo, FileType};
use core::{slice, str};
use alloc::sync::Arc;
use spin::Mutex;

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
        110 => sys_fstat(args[0], args[1] as *mut Stat),
//        111 => sys_fsync(),
//        121 => sys_getcwd(),
        128 => sys_getdirentry(args[0], args[1] as *mut DirEntry),
        130 => sys_dup(args[0], args[1]),

        // process
        001 => sys_exit(args[0] as i32),
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
    // TODO: check ptr
    info!("read: fd: {}, base: {:?}, len: {:#x}", fd, base, len);
    let slice = unsafe { slice::from_raw_parts_mut(base, len) };
    match fd {
        0 => unimplemented!(),
        1 | 2 => return -1,
        _ => {
            let file = process().files.get_mut(&fd);
            if file.is_none() {
                return -1;
            }
            let file = file.unwrap();
            file.lock().read(slice).unwrap();
        }
    }
    0
}

fn sys_write(fd: usize, base: *const u8, len: usize) -> i32 {
    // TODO: check ptr
    info!("write: fd: {}, base: {:?}, len: {:#x}", fd, base, len);
    let slice = unsafe { slice::from_raw_parts(base, len) };
    match fd {
        0 => return -1,
        1 | 2 => print!("{}", str::from_utf8(slice).unwrap()),
        _ => {
            let file = process().files.get_mut(&fd);
            if file.is_none() {
                return -1;
            }
            let file = file.unwrap();
            file.lock().write(slice).unwrap();
        }
    }
    0
}

fn sys_open(path: *const u8, flags: usize) -> i32 {
    // TODO: check ptr
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
    files.insert(fd, Arc::new(Mutex::new(file)));
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

fn sys_fstat(fd: usize, stat_ptr: *mut Stat) -> i32 {
    // TODO: check ptr
    info!("fstat: {}", fd);
    let file = process().files.get(&fd);
    if file.is_none() {
        return -1;
    }
    let file = file.unwrap();
    let stat = Stat::from(file.lock().info().unwrap());
    unsafe { stat_ptr.write(stat); }
    0
}

/// entry_id = dentry.offset / 256
/// dentry.name = entry_name
/// dentry.offset += 256
fn sys_getdirentry(fd: usize, dentry_ptr: *mut DirEntry) -> i32 {
    // TODO: check ptr
    info!("getdirentry: {}", fd);
    let file = process().files.get(&fd);
    if file.is_none() { return -1; }
    let file = file.unwrap();
    let dentry = unsafe { &mut *dentry_ptr };
    if !dentry.check() { return -1; }
    let info = file.lock().info().unwrap();
    if info.type_ != FileType::Dir || info.size <= dentry.entry_id() { return -1; }
    let name = file.lock().get_entry(dentry.entry_id());
    if name.is_err() { return -1; }
    let name = name.unwrap();
    dentry.set_name(name.as_str());
    0
}

fn sys_dup(fd1: usize, fd2: usize) -> i32 {
    info!("dup: {} {}", fd1, fd2);
    let file = process().files.get(&fd1);
    if file.is_none() {
        return -1;
    }
    let file = file.unwrap();
    if process().files.contains_key(&fd2) {
        return -1;
    }
    process().files.insert(fd2, file.clone());
    0
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
    // TODO: check ptr
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
fn sys_exit(exit_code: i32) -> i32 {
    let pid = thread::current().id();
    info!("exit: {}, code: {}", pid, exit_code);
    processor().manager().exit(pid, exit_code as usize);
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

#[repr(C)]
struct DirEntry {
    offset: u32,
    name: [u8; 256],
}

impl DirEntry {
    fn check(&self) -> bool {
        self.offset % 256 == 0
    }
    fn entry_id(&self) -> usize {
        (self.offset / 256) as usize
    }
    fn set_name(&mut self, name: &str) {
        self.name[..name.len()].copy_from_slice(name.as_bytes());
        self.name[name.len()] = 0;
        self.offset += 256;
    }
}

#[repr(C)]
struct Stat {
    /// protection mode and file type
    mode: StatMode,
    /// number of hard links
    nlinks: u32,
    /// number of blocks file is using
    blocks: u32,
    /// file size (bytes)
    size: u32,
}

bitflags! {
    struct StatMode: u32 {
        const NULL  = 0;
        /// ordinary regular file
        const FILE  = 0o10000;
        /// directory
        const DIR   = 0o20000;
        /// symbolic link
        const LINK  = 0o30000;
        /// character device
        const CHAR  = 0o40000;
        /// block device
        const BLOCK = 0o50000;
    }
}

impl From<FileInfo> for Stat {
    fn from(info: FileInfo) -> Self {
        Stat {
            mode: match info.type_ {
                FileType::File => StatMode::FILE,
                FileType::Dir => StatMode::DIR,
                _ => StatMode::NULL,
            },
            nlinks: info.nlinks as u32,
            blocks: info.blocks as u32,
            size: info.size as u32,
        }
    }
}
