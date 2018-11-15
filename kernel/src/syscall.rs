//! System call

use arch::interrupt::TrapFrame;
use process::*;
use thread;
use util;
use simple_filesystem::{INode, file::File, FileInfo, FileType};
use core::{slice, str};
use alloc::sync::Arc;
use spin::Mutex;
use alloc::vec::Vec;
use alloc::string::String;

/// System call dispatcher
pub fn syscall(id: usize, args: [usize; 6], tf: &mut TrapFrame) -> i32 {
    let ret = match id {
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
        004 => sys_exec(args[0] as *const u8, args[1] as usize, args[2] as *const *const u8, tf),
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
    };
    match ret {
        Ok(code) => code,
        Err(_) => -1,
    }
}

fn sys_read(fd: usize, base: *mut u8, len: usize) -> SysResult {
    // TODO: check ptr
    info!("read: fd: {}, base: {:?}, len: {:#x}", fd, base, len);
    let slice = unsafe { slice::from_raw_parts_mut(base, len) };
    let len = get_file(fd)?.lock().read(slice)?;
    Ok(len as i32)
}

fn sys_write(fd: usize, base: *const u8, len: usize) -> SysResult {
    // TODO: check ptr
    info!("write: fd: {}, base: {:?}, len: {:#x}", fd, base, len);
    let slice = unsafe { slice::from_raw_parts(base, len) };
    let len = get_file(fd)?.lock().write(slice)?;
    Ok(len as i32)
}

fn sys_open(path: *const u8, flags: usize) -> SysResult {
    // TODO: check ptr
    let path = unsafe { util::from_cstr(path) };
    let flags = VfsFlags::from_ucore_flags(flags);
    info!("open: path: {:?}, flags: {:?}", path, flags);
    let (fd, inode) = match path {
        "stdin:" => (0, ::fs::STDIN.clone() as Arc<INode>),
        "stdout:" => (1, ::fs::STDOUT.clone() as Arc<INode>),
        _ => {
            let fd = (3..).find(|i| !process().files.contains_key(i)).unwrap();
            let inode = ::fs::ROOT_INODE.lookup(path)?;
            (fd, inode)
        }
    };
    let file = File::new(inode, flags.contains(VfsFlags::READABLE), flags.contains(VfsFlags::WRITABLE));
    process().files.insert(fd, Arc::new(Mutex::new(file)));
    Ok(fd as i32)
}

fn sys_close(fd: usize) -> SysResult {
    info!("close: fd: {:?}", fd);
    match process().files.remove(&fd) {
        Some(_) => Ok(0),
        None => Err(SysError::InvalidFile),
    }
}

fn sys_fstat(fd: usize, stat_ptr: *mut Stat) -> SysResult {
    // TODO: check ptr
    info!("fstat: {}", fd);
    let file = get_file(fd)?;
    let stat = Stat::from(file.lock().info()?);
    unsafe { stat_ptr.write(stat); }
    Ok(0)
}

/// entry_id = dentry.offset / 256
/// dentry.name = entry_name
/// dentry.offset += 256
fn sys_getdirentry(fd: usize, dentry_ptr: *mut DirEntry) -> SysResult {
    // TODO: check ptr
    info!("getdirentry: {}", fd);
    let file = get_file(fd)?;
    let dentry = unsafe { &mut *dentry_ptr };
    if !dentry.check() {
        return Err(SysError::InvalidArgument);
    }
    let info = file.lock().info()?;
    if info.type_ != FileType::Dir || info.size <= dentry.entry_id() {
        return Err(SysError::InvalidArgument);
    }
    let name = file.lock().get_entry(dentry.entry_id())?;
    dentry.set_name(name.as_str());
    Ok(0)
}

fn sys_dup(fd1: usize, fd2: usize) -> SysResult {
    info!("dup: {} {}", fd1, fd2);
    let file = get_file(fd1)?;
    if process().files.contains_key(&fd2) {
        return Err(SysError::InvalidFile);
    }
    process().files.insert(fd2, file.clone());
    Ok(0)
}

/// Fork the current process. Return the child's PID.
fn sys_fork(tf: &TrapFrame) -> SysResult {
    let mut context = process().fork(tf);
    let pid = processor().manager().add(context);
    Process::new_fork(pid, thread::current().id());
    info!("fork: {} -> {}", thread::current().id(), pid);
    Ok(pid as i32)
}

/// Wait the process exit.
/// Return the PID. Store exit code to `code` if it's not null.
fn sys_wait(pid: usize, code: *mut i32) -> SysResult {
    // TODO: check ptr
    loop {
        let wait_procs = match pid {
            0 => Process::get_children(),
            _ => vec![pid],
        };
        if wait_procs.is_empty() {
            return Ok(-1);
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
                    return Ok(0);
                }
                None => return Ok(-1),
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

fn sys_exec(name: *const u8, argc: usize, argv: *const *const u8, tf: &mut TrapFrame) -> SysResult {
    // TODO: check ptr
    let name = if name.is_null() { "" } else { unsafe { util::from_cstr(name) } };
    info!("exec: {:?}, argc: {}, argv: {:?}", name, argc, argv);
    // Copy args to kernel
    let args: Vec<String> = unsafe {
        slice::from_raw_parts(argv, argc).iter()
            .map(|&arg| String::from(util::from_cstr(arg)))
            .collect()
    };

    // Read program file
    let path = args[0].as_str();
    let inode = ::fs::ROOT_INODE.lookup(path)?;
    let size = inode.info()?.size;
    let mut buf = Vec::with_capacity(size);
    unsafe { buf.set_len(size); }
    inode.read_at(0, buf.as_mut_slice())?;

    // Make new Context
    let iter = args.iter().map(|s| s.as_str());
    let mut context = ContextImpl::new_user(buf.as_slice(), iter);

    // Activate new page table
    unsafe { context.memory_set.activate(); }

    // Modify the TrapFrame
    *tf = unsafe { context.arch.get_init_tf() };

    // Swap Context but keep KStack
    ::core::mem::swap(&mut process().kstack, &mut context.kstack);
    ::core::mem::swap(process(), &mut *context);

    Ok(0)
}

fn sys_yield() -> SysResult {
    thread::yield_now();
    Ok(0)
}

/// Kill the process
fn sys_kill(pid: usize) -> SysResult {
    info!("kill: {}", pid);
    processor().manager().exit(pid, 0x100);
    if pid == thread::current().id() {
        processor().yield_now();
    }
    Ok(0)
}

/// Get the current process id
fn sys_getpid() -> SysResult {
    Ok(thread::current().id() as i32)
}

/// Exit the current process
fn sys_exit(exit_code: i32) -> SysResult {
    let pid = thread::current().id();
    info!("exit: {}, code: {}", pid, exit_code);
    processor().manager().exit(pid, exit_code as usize);
    processor().yield_now();
    unreachable!();
}

fn sys_sleep(time: usize) -> SysResult {
    use core::time::Duration;
    thread::sleep(Duration::from_millis(time as u64 * 10));
    Ok(0)
}

fn sys_get_time() -> SysResult {
    unsafe { Ok(::trap::TICK as i32) }
}

fn sys_lab6_set_priority(priority: usize) -> SysResult {
    let pid = thread::current().id();
    processor().manager().set_priority(pid, priority as u8);
    Ok(0)
}

fn sys_putc(c: char) -> SysResult {
    print!("{}", c);
    Ok(0)
}

fn get_file(fd: usize) -> Result<&'static Arc<Mutex<File>>, SysError> {
    process().files.get(&fd).ok_or(SysError::InvalidFile)
}

pub type SysResult = Result<i32, SysError>;

#[repr(i32)]
pub enum SysError {
    VfsError,
    InvalidFile,
    InvalidArgument,
}

impl From<()> for SysError {
    fn from(_: ()) -> Self {
        SysError::VfsError
    }
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
