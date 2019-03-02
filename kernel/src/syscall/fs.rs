//! Syscalls for file system

use rcore_fs::vfs::Timespec;

use crate::fs::*;
use crate::memory::MemorySet;

use super::*;
use super::net::*;

pub fn sys_read(fd: usize, base: *mut u8, len: usize) -> SysResult {
    info!("read: fd: {}, base: {:?}, len: {:#x}", fd, base, len);
    let mut proc = process();
    if !proc.memory_set.check_mut_array(base, len) {
        return Err(SysError::EFAULT);
    }
    let slice = unsafe { slice::from_raw_parts_mut(base, len) };
    let len = proc.get_file(fd)?.read(slice)?;
    Ok(len as isize)
}

pub fn sys_write(fd: usize, base: *const u8, len: usize) -> SysResult {
    info!("write: fd: {}, base: {:?}, len: {:#x}", fd, base, len);
    let mut proc = process();
    if !proc.memory_set.check_array(base, len) {
        return Err(SysError::EFAULT);
    }
    match proc.files.get(&fd) {
        Some(FileLike::File(_)) => sys_write_file(&mut proc, fd, base, len),
        Some(FileLike::Socket(_)) => sys_write_socket(&mut proc, fd, base, len),
        None => Err(SysError::EINVAL)
    }
}

pub fn sys_write_file(proc: &mut Process, fd: usize, base: *const u8, len: usize) -> SysResult {
    let slice = unsafe { slice::from_raw_parts(base, len) };
    let len = proc.get_file(fd)?.write(slice)?;
    Ok(len as isize)
}

pub fn sys_readv(fd: usize, iov_ptr: *const IoVec, iov_count: usize) -> SysResult {
    info!("readv: fd: {}, iov: {:?}, count: {}", fd, iov_ptr, iov_count);
    let mut proc = process();
    let mut iovs = IoVecs::check_and_new(iov_ptr, iov_count, &proc.memory_set, true)?;

    // read all data to a buf
    let file = proc.get_file(fd)?;
    let mut buf = iovs.new_buf(true);
    let len = file.read(buf.as_mut_slice())?;
    // copy data to user
    iovs.write_all_from_slice(&buf[..len]);
    Ok(len as isize)
}

pub fn sys_writev(fd: usize, iov_ptr: *const IoVec, iov_count: usize) -> SysResult {
    info!("writev: fd: {}, iov: {:?}, count: {}", fd, iov_ptr, iov_count);
    let mut proc = process();
    let iovs = IoVecs::check_and_new(iov_ptr, iov_count, &proc.memory_set,  false)?;

    let file = proc.get_file(fd)?;
    let buf = iovs.read_all_to_vec();
    let len = file.write(buf.as_slice())?;
    Ok(len as isize)
}

pub fn sys_open(path: *const u8, flags: usize, mode: usize) -> SysResult {
    let mut proc = process();
    let path = unsafe { proc.memory_set.check_and_clone_cstr(path) }
        .ok_or(SysError::EFAULT)?;
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
                    return Err(SysError::EEXIST);
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

    let fd = proc.get_free_inode();

    let file = FileHandle::new(inode, flags.to_options());
    proc.files.insert(fd, FileLike::File(file));
    Ok(fd as isize)
}

pub fn sys_close(fd: usize) -> SysResult {
    info!("close: fd: {:?}", fd);
    let mut proc = process();
    match proc.files.remove(&fd) {
        Some(FileLike::File(_)) => Ok(0),
        Some(FileLike::Socket(handle)) => sys_close_socket(&mut proc, fd, handle),
        None => Err(SysError::EINVAL),
    }
}

pub fn sys_stat(path: *const u8, stat_ptr: *mut Stat) -> SysResult {
    warn!("stat is partial implemented as lstat");
    sys_lstat(path, stat_ptr)
}

pub fn sys_fstat(fd: usize, stat_ptr: *mut Stat) -> SysResult {
    info!("fstat: fd: {}", fd);
    let mut proc = process();
    if !proc.memory_set.check_mut_ptr(stat_ptr) {
        return Err(SysError::EFAULT);
    }
    let file = proc.get_file(fd)?;
    let stat = Stat::from(file.info()?);
    // TODO: handle symlink
    unsafe { stat_ptr.write(stat); }
    Ok(0)
}

pub fn sys_lstat(path: *const u8, stat_ptr: *mut Stat) -> SysResult {
    let mut proc = process();
    let path = unsafe { proc.memory_set.check_and_clone_cstr(path) }
        .ok_or(SysError::EFAULT)?;
    if !proc.memory_set.check_mut_ptr(stat_ptr) {
        return Err(SysError::EFAULT);
    }
    info!("lstat: path: {}", path);

    let inode = ROOT_INODE.lookup(path.as_str())?;
    let stat = Stat::from(inode.metadata()?);
    unsafe { stat_ptr.write(stat); }
    Ok(0)
}

pub fn sys_lseek(fd: usize, offset: i64, whence: u8) -> SysResult {
    let pos = match whence {
        SEEK_SET => SeekFrom::Start(offset as u64),
        SEEK_END => SeekFrom::End(offset),
        SEEK_CUR => SeekFrom::Current(offset),
        _ => return Err(SysError::EINVAL),
    };
    info!("lseek: fd: {}, pos: {:?}", fd, pos);

    let mut proc = process();
    let file = proc.get_file(fd)?;
    let offset = file.seek(pos)?;
    Ok(offset as isize)
}

pub fn sys_getdents64(fd: usize, buf: *mut LinuxDirent64, buf_size: usize) -> SysResult {
    info!("getdents64: fd: {}, ptr: {:?}, buf_size: {}", fd, buf, buf_size);
    let mut proc = process();
    if !proc.memory_set.check_mut_array(buf as *mut u8, buf_size) {
        return Err(SysError::EFAULT);
    }
    let file = proc.get_file(fd)?;
    let info = file.info()?;
    if info.type_ != FileType::Dir {
        return Err(SysError::ENOTDIR);
    }
    let mut writer = unsafe { DirentBufWriter::new(buf, buf_size) };
    loop {
        let name = match file.read_entry() {
            Err(FsError::EntryNotFound) => break,
            r => r,
        }?;
        let ok = writer.try_write(0, 0, name.as_str());
        if !ok { break; }
    }
    Ok(writer.written_size as isize)
}

pub fn sys_dup2(fd1: usize, fd2: usize) -> SysResult {
    info!("dup2: {} {}", fd1, fd2);
    let mut proc = process();
    if proc.files.contains_key(&fd2) {
        return Err(SysError::EINVAL);
    }
    let file = proc.get_file(fd1)?.clone();
    proc.files.insert(fd2, FileLike::File(file));
    Ok(0)
}


impl Process {
    fn get_file(&mut self, fd: usize) -> Result<&mut FileHandle, SysError> {
        self.files.get_mut(&fd).ok_or(SysError::EBADF).and_then(|f| {
            match f {
                FileLike::File(file) => Ok(file),
                _ => Err(SysError::EBADF)
            }
        })
    }
}

impl From<FsError> for SysError {
    fn from(error: FsError) -> Self {
        match error {
            FsError::NotSupported => SysError::ENOSYS,
            FsError::NotFile => SysError::EISDIR,
            FsError::IsDir => SysError::EISDIR,
            FsError::NotDir => SysError::ENOTDIR,
            FsError::EntryNotFound => SysError::ENOENT,
            FsError::EntryExist => SysError::EEXIST,
            FsError::NotSameFs => SysError::EXDEV,
            FsError::InvalidParam => SysError::EINVAL,
            FsError::NoDeviceSpace => SysError::ENOMEM,
            FsError::DirRemoved => SysError::ENOENT,
            FsError::DirNotEmpty => SysError::ENOTEMPTY,
            FsError::WrongFs => SysError::EINVAL,
            FsError::DeviceError => SysError::EIO,
        }
    }
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

#[derive(Debug)]
#[repr(packed)] // Don't use 'C'. Or its size will align up to 8 bytes.
pub struct LinuxDirent64 {
    /// Inode number
    ino: u64,
    /// Offset to next structure
    offset: u64,
    /// Size of this dirent
    reclen: u16,
    /// File type
    type_: u8,
    /// Filename (null-terminated)
    name: [u8; 0],
}

struct DirentBufWriter {
    ptr: *mut LinuxDirent64,
    rest_size: usize,
    written_size: usize,
}

impl DirentBufWriter {
    unsafe fn new(buf: *mut LinuxDirent64, size: usize) -> Self {
        DirentBufWriter {
            ptr: buf,
            rest_size: size,
            written_size: 0,
        }
    }
    fn try_write(&mut self, inode: u64, type_: u8, name: &str) -> bool {
        let len = ::core::mem::size_of::<LinuxDirent64>() + name.len() + 1;
        let len = (len + 7) / 8 * 8; // align up
        if self.rest_size < len {
            return false;
        }
        let dent = LinuxDirent64 {
            ino: inode,
            offset: 0,
            reclen: len as u16,
            type_,
            name: [],
        };
        unsafe {
            self.ptr.write(dent);
            let name_ptr = self.ptr.add(1) as _;
            util::write_cstr(name_ptr, name);
            self.ptr = (self.ptr as *const u8).add(len) as _;
        }
        self.rest_size -= len;
        self.written_size += len;
        true
    }
}

#[repr(C)]
pub struct Stat {
    /// ID of device containing file
    dev: u64,
    /// inode number
    ino: u64,
    /// number of hard links
    nlink: u64,

    /// file type and mode
    mode: StatMode,
    /// user ID of owner
    uid: u32,
    /// group ID of owner
    gid: u32,
    /// padding
    _pad0: u32,
    /// device ID (if special file)
    rdev: u64,
    /// total size, in bytes
    size: u64,
    /// blocksize for filesystem I/O
    blksize: u64,
    /// number of 512B blocks allocated
    blocks: u64,

    /// last access time
    atime: Timespec,
    /// last modification time
    mtime: Timespec,
    /// last status change time
    ctime: Timespec,
}

bitflags! {
    pub struct StatMode: u32 {
        const NULL  = 0;
        /// Type
        const TYPE_MASK = 0o170000;
        /// FIFO
        const FIFO  = 0o010000;
        /// character device
        const CHAR  = 0o020000;
        /// directory
        const DIR   = 0o040000;
        /// block device
        const BLOCK = 0o060000;
        /// ordinary regular file
        const FILE  = 0o100000;
        /// symbolic link
        const LINK  = 0o120000;
        /// socket
        const SOCKET = 0o140000;

        /// Set-user-ID on execution.
        const SET_UID = 0o4000;
        /// Set-group-ID on execution.
        const SET_GID = 0o2000;

        /// Read, write, execute/search by owner.
        const OWNER_MASK = 0o700;
        /// Read permission, owner.
        const OWNER_READ = 0o400;
        /// Write permission, owner.
        const OWNER_WRITE = 0o200;
        /// Execute/search permission, owner.
        const OWNER_EXEC = 0o100;

        /// Read, write, execute/search by group.
        const GROUP_MASK = 0o70;
        /// Read permission, group.
        const GROUP_READ = 0o40;
        /// Write permission, group.
        const GROUP_WRITE = 0o20;
        /// Execute/search permission, group.
        const GROUP_EXEC = 0o10;

        /// Read, write, execute/search by others.
        const OTHER_MASK = 0o7;
        /// Read permission, others.
        const OTHER_READ = 0o4;
        /// Write permission, others.
        const OTHER_WRITE = 0o2;
        /// Execute/search permission, others.
        const OTHER_EXEC = 0o1;
    }
}

impl StatMode {
    fn from_type_mode(type_: FileType, mode: u16) -> Self {
        let type_ = match type_ {
            FileType::File => StatMode::FILE,
            FileType::Dir => StatMode::DIR,
            FileType::SymLink => StatMode::LINK,
            FileType::CharDevice => StatMode::CHAR,
            FileType::BlockDevice => StatMode::BLOCK,
            FileType::Socket => StatMode::SOCKET,
            FileType::NamedPipe => StatMode::FIFO,
            _ => StatMode::NULL,
        };
        let mode = StatMode::from_bits_truncate(mode as u32);
        type_ | mode
    }
}

impl From<Metadata> for Stat {
    fn from(info: Metadata) -> Self {
        Stat {
            dev: info.dev as u64,
            ino: info.inode as u64,
            mode: StatMode::from_type_mode(info.type_, info.mode as u16),
            nlink: info.nlinks as u64,
            uid: info.uid as u32,
            gid: info.gid as u32,
            rdev: 0,
            size: info.size as u64,
            blksize: info.blk_size as u64,
            blocks: info.blocks as u64,
            atime: info.atime,
            mtime: info.mtime,
            ctime: info.ctime,
            _pad0: 0
        }
    }
}

const SEEK_SET: u8 = 1;
const SEEK_CUR: u8 = 2;
const SEEK_END: u8 = 4;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct IoVec {
    /// Starting address
    base: *mut u8,
    /// Number of bytes to transfer
    len: u64,
}

/// A valid IoVecs request from user
#[derive(Debug)]
struct IoVecs(Vec<&'static mut [u8]>);

impl IoVecs {
    fn check_and_new(iov_ptr: *const IoVec, iov_count: usize, vm: &MemorySet, readv: bool) -> Result<Self, SysError> {
        if !vm.check_array(iov_ptr, iov_count) {
            return Err(SysError::EFAULT);
        }
        let iovs = unsafe { slice::from_raw_parts(iov_ptr, iov_count) }.to_vec();
        // check all bufs in iov
        for iov in iovs.iter() {
            if readv && !vm.check_mut_array(iov.base, iov.len as usize)
                || !readv && !vm.check_array(iov.base, iov.len as usize) {
                return Err(SysError::EFAULT);
            }
        }
        let slices = iovs.iter().map(|iov| unsafe { slice::from_raw_parts_mut(iov.base, iov.len as usize) }).collect();
        Ok(IoVecs(slices))
    }

    fn read_all_to_vec(&self) -> Vec<u8> {
        let mut buf = self.new_buf(false);
        for slice in self.0.iter() {
            buf.extend(slice.iter());
        }
        buf
    }

    fn write_all_from_slice(&mut self, buf: &[u8]) {
        let mut copied_len = 0;
        for slice in self.0.iter_mut() {
            slice.copy_from_slice(&buf[copied_len..copied_len + slice.len()]);
            copied_len += slice.len();
        }
    }

    /// Create a new Vec buffer from IoVecs
    /// For readv:  `set_len` is true,  Vec.len = total_len.
    /// For writev: `set_len` is false, Vec.cap = total_len.
    fn new_buf(&self, set_len: bool) -> Vec<u8> {
        let total_len = self.0.iter().map(|slice| slice.len()).sum::<usize>();
        let mut buf = Vec::with_capacity(total_len);
        if set_len {
            unsafe { buf.set_len(total_len); }
        }
        buf
    }
}
