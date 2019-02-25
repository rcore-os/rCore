//! Syscalls for file system

use super::*;

use crate::fs::{ROOT_INODE, OpenOptions};

pub fn sys_read(fd: usize, base: *mut u8, len: usize) -> SysResult {
    info!("read: fd: {}, base: {:?}, len: {:#x}", fd, base, len);
    let mut proc = process();
    if !proc.memory_set.check_mut_array(base, len) {
        return Err(SysError::Inval);
    }
    let slice = unsafe { slice::from_raw_parts_mut(base, len) };
    let len = get_file(&mut proc, fd)?.read(slice)?;
    Ok(len as isize)
}

pub fn sys_write(fd: usize, base: *const u8, len: usize) -> SysResult {
    info!("write: fd: {}, base: {:?}, len: {:#x}", fd, base, len);
    let mut proc = process();
    if !proc.memory_set.check_array(base, len) {
        return Err(SysError::Inval);
    }
    let slice = unsafe { slice::from_raw_parts(base, len) };
    let len = get_file(&mut proc, fd)?.write(slice)?;
    Ok(len as isize)
}

pub fn sys_open(path: *const u8, flags: usize, mode: usize) -> SysResult {
    let mut proc = process();
    let path = unsafe { proc.memory_set.check_and_clone_cstr(path) }
        .ok_or(SysError::Inval)?;
    let flags = OpenFlags::from_bits_truncate(flags);
    info!("open: path: {:?}, flags: {:?}, mode: {:#o}", path, flags, mode);

    let inode =
    if flags.contains(OpenFlags::CREATE) {
        // FIXME: assume path start from root now
        let mut split = path.as_str().rsplitn(2, '/');
        let file_name = split.next().unwrap();
        let dir_path = split.next().unwrap_or(".");
        let dir_inode = ROOT_INODE.lookup(dir_path)?;
        match dir_inode.find(file_name) {
            Ok(file_inode) => {
                if flags.contains(OpenFlags::EXCLUSIVE) {
                    return Err(SysError::Exists);
                }
                file_inode
            },
            Err(FsError::EntryNotFound) => {
                dir_inode.create(file_name, FileType::File, mode as u32)?
            }
            Err(e) => return Err(SysError::from(e)),
        }
    } else {
        // TODO: remove "stdin:" "stdout:"
        match path.as_str() {
            "stdin:" => crate::fs::STDIN.clone() as Arc<INode>,
            "stdout:" => crate::fs::STDOUT.clone() as Arc<INode>,
            _ => ROOT_INODE.lookup(path.as_str())?,
        }
    };

    let fd = (3..).find(|i| !proc.files.contains_key(i)).unwrap();

    let file = FileHandle::new(inode, flags.to_options());
    proc.files.insert(fd, file);
    Ok(fd as isize)
}

pub fn sys_close(fd: usize) -> SysResult {
    info!("close: fd: {:?}", fd);
    match process().files.remove(&fd) {
        Some(_) => Ok(0),
        None => Err(SysError::Inval),
    }
}

pub fn sys_fstat(fd: usize, stat_ptr: *mut Stat) -> SysResult {
    info!("fstat: {}", fd);
    let mut proc = process();
    if !proc.memory_set.check_mut_ptr(stat_ptr) {
        return Err(SysError::Inval);
    }
    let file = get_file(&mut proc, fd)?;
    let stat = Stat::from(file.info()?);
    unsafe { stat_ptr.write(stat); }
    Ok(0)
}

/// entry_id = dentry.offset / 256
/// dentry.name = entry_name
/// dentry.offset += 256
pub fn sys_getdirentry(fd: usize, dentry_ptr: *mut DirEntry) -> SysResult {
    info!("getdirentry: {}", fd);
    let mut proc = process();
    if !proc.memory_set.check_mut_ptr(dentry_ptr) {
        return Err(SysError::Inval);
    }
    let file = get_file(&mut proc, fd)?;
    let dentry = unsafe { &mut *dentry_ptr };
    if !dentry.check() {
        return Err(SysError::Inval);
    }
    let info = file.info()?;
    if info.type_ != FileType::Dir || info.size <= dentry.entry_id() {
        return Err(SysError::Inval);
    }
    let name = file.get_entry(dentry.entry_id())?;
    dentry.set_name(name.as_str());
    Ok(0)
}

pub fn sys_dup2(fd1: usize, fd2: usize) -> SysResult {
    info!("dup2: {} {}", fd1, fd2);
    let mut proc = process();
    if proc.files.contains_key(&fd2) {
        return Err(SysError::Inval);
    }
    let file = get_file(&mut proc, fd1)?.clone();
    proc.files.insert(fd2, file);
    Ok(0)
}

fn get_file<'a>(proc: &'a mut MutexGuard<'static, Process>, fd: usize) -> Result<&'a mut FileHandle, SysError> {
    proc.files.get_mut(&fd).ok_or(SysError::Inval)
}

bitflags! {
    struct OpenFlags: usize {
        /// read only
        const RDONLY = 0;
        /// write only
        const WRONLY = 1;
        /// read write
        const RDWR = 2;
        /// create file if it does not exist
        const CREATE = 1 << 6;
        /// error if CREATE and the file exists
        const EXCLUSIVE = 1 << 7;
        /// truncate file upon open
        const TRUNCATE = 1 << 9;
        /// append on each write
        const APPEND = 1 << 10;
    }
}

impl OpenFlags {
    fn readable(&self) -> bool {
        let b = self.bits() & 0b11;
        b == OpenFlags::RDONLY.bits() || b == OpenFlags::RDWR.bits()
    }
    fn writable(&self) -> bool {
        let b = self.bits() & 0b11;
        b == OpenFlags::WRONLY.bits() || b == OpenFlags::RDWR.bits()
    }
    fn to_options(&self) -> OpenOptions {
        OpenOptions {
            read: self.readable(),
            write: self.writable(),
            append: self.contains(OpenFlags::APPEND),
        }
    }
}

#[repr(C)]
pub struct DirEntry {
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
pub struct Stat {
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
    pub struct StatMode: u32 {
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

impl From<Metadata> for Stat {
    fn from(info: Metadata) -> Self {
        Stat {
            mode: match info.type_ {
                FileType::File => StatMode::FILE,
                FileType::Dir => StatMode::DIR,
                FileType::SymLink => StatMode::LINK,
                FileType::CharDevice => StatMode::CHAR,
                FileType::BlockDevice => StatMode::BLOCK,
                _ => StatMode::NULL,
                //Note: we should mark FileType as #[non_exhaustive]
                //      but it is currently not implemented for enum
                //      see rust-lang/rust#44109
            },
            nlinks: info.nlinks as u32,
            blocks: info.blocks as u32,
            size: info.size as u32,
        }
    }
}
