//! Syscalls for file system

use core::mem::size_of;
use core::cmp::min;
use rcore_fs::vfs::Timespec;
use smoltcp::socket::*;

use crate::fs::*;
use crate::memory::MemorySet;
use crate::sync::Condvar;
use crate::drivers::{NET_DRIVERS, SOCKET_ACTIVITY};

use super::*;
use super::net::*;

pub fn sys_read(fd: usize, base: *mut u8, len: usize) -> SysResult {
    info!("read: fd: {}, base: {:?}, len: {:#x}", fd, base, len);
    let mut proc = process();
    proc.memory_set.check_mut_array(base, len)?;
    match proc.files.get(&fd) {
        Some(FileLike::File(_)) => sys_read_file(&mut proc, fd, base, len),
        Some(FileLike::Socket(_)) => sys_read_socket(&mut proc, fd, base, len),
        None => Err(SysError::EINVAL)
    }
}

pub fn sys_write(fd: usize, base: *const u8, len: usize) -> SysResult {
    info!("write: fd: {}, base: {:?}, len: {:#x}", fd, base, len);
    let mut proc = process();
    proc.memory_set.check_array(base, len)?;

    match proc.files.get(&fd) {
        Some(FileLike::File(_)) => sys_write_file(&mut proc, fd, base, len),
        Some(FileLike::Socket(_)) => sys_write_socket(&mut proc, fd, base, len),
        None => Err(SysError::EINVAL)
    }
}

pub fn sys_pread(fd: usize, base: *mut u8, len: usize, offset: usize) -> SysResult {
    info!("pread: fd: {}, base: {:?}, len: {}, offset: {}", fd, base, len, offset);
    let mut proc = process();
    proc.memory_set.check_mut_array(base, len)?;

    let slice = unsafe { slice::from_raw_parts_mut(base, len) };
    let len = proc.get_file(fd)?.read_at(offset, slice)?;
    Ok(len)
}

pub fn sys_pwrite(fd: usize, base: *const u8, len: usize, offset: usize) -> SysResult {
    info!("pwrite: fd: {}, base: {:?}, len: {}, offset: {}", fd, base, len, offset);
    let mut proc = process();
    proc.memory_set.check_array(base, len)?;

    let slice = unsafe { slice::from_raw_parts(base, len) };
    let len = proc.get_file(fd)?.write_at(offset, slice)?;
    Ok(len)
}

pub fn sys_read_file(proc: &mut Process, fd: usize, base: *mut u8, len: usize) -> SysResult {
    let slice = unsafe { slice::from_raw_parts_mut(base, len) };
    let len = proc.get_file(fd)?.read(slice)?;
    Ok(len)
}

pub fn sys_write_file(proc: &mut Process, fd: usize, base: *const u8, len: usize) -> SysResult {
    let slice = unsafe { slice::from_raw_parts(base, len) };
    let len = proc.get_file(fd)?.write(slice)?;
    Ok(len)
}

pub fn sys_poll(ufds: *mut PollFd, nfds: usize, timeout_msecs: usize) -> SysResult {
    info!("poll: ufds: {:?}, nfds: {}, timeout_msecs: {:#x}", ufds, nfds, timeout_msecs);
    let mut proc = process();
    proc.memory_set.check_mut_array(ufds, nfds)?;

    let polls = unsafe { slice::from_raw_parts_mut(ufds, nfds) };
    for poll in polls.iter() {
        if proc.files.get(&(poll.fd as usize)).is_none() {
            return Err(SysError::EINVAL);
        }
    }
    drop(proc);

    let begin_time_ms = crate::trap::uptime_msec();
    loop {
        use PollEvents as PE;
        let mut proc = process();
        let mut events = 0;
        for poll in polls.iter_mut() {
            poll.revents = PE::NONE;
            match proc.files.get(&(poll.fd as usize)) {
                Some(FileLike::File(_)) => {
                    // FIXME: assume it is stdin for now
                    if poll.events.contains(PE::IN) && STDIN.can_read() {
                        poll.revents = poll.revents | PE::IN;
                        events = events + 1;
                    }
                },
                Some(FileLike::Socket(wrapper)) => {
                    let (input, output, err) = poll_socket(&wrapper);
                    if err {
                        poll.revents = poll.revents | PE::HUP;
                        events = events + 1;
                    }
                    if input && poll.events.contains(PE::IN) {
                        poll.revents = poll.revents | PE::IN;
                        events = events + 1;
                    }
                    if output && poll.events.contains(PE::OUT) {
                        poll.revents = poll.revents | PE::OUT;
                        events = events + 1;
                    }
                }
                None => {
                    poll.revents = poll.revents | PE::ERR;
                    events = events + 1;
                }
            }
        }
        drop(proc);

        if events > 0 {
            return Ok(events);
        }

        let current_time_ms = crate::trap::uptime_msec();
        if timeout_msecs < (1 << 31) && current_time_ms - begin_time_ms > timeout_msecs {
            return Ok(0);
        }

        Condvar::wait_any(&[&STDIN.pushed, &(*SOCKET_ACTIVITY)]);
    }
}

const FD_PER_ITEM: usize = 8 * size_of::<u32>();
const MAX_FDSET_SIZE: usize = 1024 / FD_PER_ITEM;

struct FdSet {
    addr: *mut u32,
    nfds: usize,
    saved: [u32; MAX_FDSET_SIZE]
}

impl FdSet {
    /// Initialize a `FdSet` from pointer and number of fds
    /// Check if the array is large enough
    fn new(proc: &Process, addr: *mut u32, nfds: usize) -> Result<FdSet, SysError> {
        let mut saved = [0u32; MAX_FDSET_SIZE];
        if addr as usize != 0 {
            let len = (nfds + FD_PER_ITEM - 1) / FD_PER_ITEM;
            proc.memory_set.check_mut_array(addr, len)?;
            if len > MAX_FDSET_SIZE {
                return Err(SysError::EINVAL);
            }
            let slice = unsafe {slice::from_raw_parts_mut(addr, len)};

            // save the fdset, and clear it
            for i in 0..len {
                saved[i] = slice[i];
                slice[i] = 0;
            }
        }

        Ok(FdSet {
            addr,
            nfds,
            saved
        })
    }

    /// Try to set fd in `FdSet`
    /// Return true when `FdSet` is valid, and false when `FdSet` is bad (i.e. null pointer)
    /// Fd should be less than nfds
    fn set(&mut self, fd: usize) -> bool {
        if self.addr as usize != 0 {
            assert!(fd < self.nfds);
            unsafe {
                *self.addr.add(fd / 8 / size_of::<u32>()) |= 1 << (fd % (8 * size_of::<u32>()));
            }
            true
        } else {
            false
        }
    }

    /// Check to see fd is see in original `FdSet`
    /// Fd should be less than nfds
    fn is_set(&mut self, fd: usize) -> bool {
        assert!(fd < self.nfds);
        self.saved[fd / 8 / size_of::<u32>()] & (1 << (fd % (8 * size_of::<u32>()))) != 0
    }
}

pub fn sys_select(nfds: usize, read: *mut u32, write: *mut u32, err: *mut u32, timeout: *const TimeVal) -> SysResult {
    info!("select: nfds: {}, read: {:?}, write: {:?}, err: {:?}, timeout: {:?}", nfds, read, write, err, timeout);

    let mut proc = process();
    let mut read_fds = FdSet::new(&proc, read, nfds)?;
    let mut write_fds = FdSet::new(&proc, write, nfds)?;
    let mut err_fds = FdSet::new(&proc, err, nfds)?;
    let timeout_msecs = if timeout as usize != 0 {
        proc.memory_set.check_ptr(timeout)?;
        unsafe { *timeout }.to_msec()
    } else {
        // infinity
        1 << 31
    };
    drop(proc);

    let begin_time_ms = crate::trap::uptime_msec();
    loop {
        let mut proc = process();
        let mut events = 0;
        for (fd, file) in proc.files.iter() {
            if *fd < nfds {
                match file {
                    FileLike::File(_) => {
                        // FIXME: assume it is stdin for now
                        if STDIN.can_read() {
                            if read_fds.is_set(*fd){
                                read_fds.set(*fd);
                                events = events + 1;
                            }
                        }
                    },
                    FileLike::Socket(wrapper) => {
                        let (input, output, err) = poll_socket(&wrapper);
                        if err && err_fds.is_set(*fd){
                            err_fds.set(*fd);
                            events = events + 1;
                        }
                        if input && read_fds.is_set(*fd){
                            read_fds.set(*fd);
                            events = events + 1;
                        }
                        if output && write_fds.is_set(*fd){
                            write_fds.set(*fd);
                            events = events + 1;
                        }
                    }
                }
            }
        }
        drop(proc);

        if events > 0 {
            return Ok(events);
        }

        let current_time_ms = crate::trap::uptime_msec();
        if timeout_msecs < (1 << 31) && current_time_ms - begin_time_ms > timeout_msecs as usize {
            return Ok(0);
        }

        Condvar::wait_any(&[&STDIN.pushed, &(*SOCKET_ACTIVITY)]);
    }
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
    Ok(len)
}

pub fn sys_writev(fd: usize, iov_ptr: *const IoVec, iov_count: usize) -> SysResult {
    info!("writev: fd: {}, iov: {:?}, count: {}", fd, iov_ptr, iov_count);
    let mut proc = process();
    let iovs = IoVecs::check_and_new(iov_ptr, iov_count, &proc.memory_set, false)?;

    let buf = iovs.read_all_to_vec();
    let len = buf.len();

    match proc.files.get(&fd) {
        Some(FileLike::File(_)) => sys_write_file(&mut proc, fd, buf.as_ptr(), len),
        Some(FileLike::Socket(_)) => sys_write_socket(&mut proc, fd, buf.as_ptr(), len),
        None => Err(SysError::EINVAL)
    }
}

pub fn sys_open(path: *const u8, flags: usize, mode: usize) -> SysResult {
    let mut proc = process();
    let path = unsafe { proc.memory_set.check_and_clone_cstr(path)? };
    let flags = OpenFlags::from_bits_truncate(flags);
    info!("open: path: {:?}, flags: {:?}, mode: {:#o}", path, flags, mode);

    let inode =
    if flags.contains(OpenFlags::CREATE) {
        let (dir_path, file_name) = split_path(&path);
        let dir_inode = proc.lookup_inode(dir_path)?;
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
            _ => proc.lookup_inode(&path)?,
        }
    };

    let fd = proc.get_free_inode();

    let file = FileHandle::new(inode, flags.to_options());
    proc.files.insert(fd, FileLike::File(file));
    Ok(fd)
}

pub fn sys_close(fd: usize) -> SysResult {
    info!("close: fd: {:?}", fd);
    let mut proc = process();
    sys_close_internal(&mut proc, fd)
}

pub fn sys_close_internal(proc: &mut Process, fd: usize) -> SysResult {
    match proc.files.remove(&fd) {
        Some(FileLike::File(_)) => Ok(0),
        Some(FileLike::Socket(wrapper)) => sys_close_socket(proc, fd, wrapper.handle),
        None => Err(SysError::EINVAL),
    }
}

pub fn sys_access(path: *const u8, mode: usize) -> SysResult {
    info!("access: path: {:?}, mode: {}", path, mode);
    // TODO: check permissions based on uid/git
    Ok(0)
}

pub fn sys_getcwd(buf: *mut u8, len: usize) -> SysResult {
    info!("getcwd: buf: {:?}, len: {:#x}", buf, len);
    let mut proc = process();
    proc.memory_set.check_mut_array(buf, len)?;
    if proc.cwd.len() + 1 > len {
        return Err(SysError::ERANGE);
    }
    unsafe {
        util::write_cstr(buf, &proc.cwd)
    }
    Ok(0)
}

pub fn sys_stat(path: *const u8, stat_ptr: *mut Stat) -> SysResult {
    warn!("stat is partial implemented as lstat");
    sys_lstat(path, stat_ptr)
}

pub fn sys_fstat(fd: usize, stat_ptr: *mut Stat) -> SysResult {
    info!("fstat: fd: {}", fd);
    let mut proc = process();
    proc.memory_set.check_mut_ptr(stat_ptr)?;
    let file = proc.get_file(fd)?;
    let stat = Stat::from(file.metadata()?);
    // TODO: handle symlink
    unsafe { stat_ptr.write(stat); }
    Ok(0)
}

pub fn sys_lstat(path: *const u8, stat_ptr: *mut Stat) -> SysResult {
    let mut proc = process();
    let path = unsafe { proc.memory_set.check_and_clone_cstr(path)? };
    proc.memory_set.check_mut_ptr(stat_ptr)?;
    info!("lstat: path: {}", path);

    let inode = proc.lookup_inode(&path)?;
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
    Ok(offset as usize)
}

pub fn sys_fsync(fd: usize) -> SysResult {
    info!("fsync: fd: {}", fd);
    process().get_file(fd)?.sync_all()?;
    Ok(0)
}

pub fn sys_fdatasync(fd: usize) -> SysResult {
    info!("fdatasync: fd: {}", fd);
    process().get_file(fd)?.sync_data()?;
    Ok(0)
}

pub fn sys_truncate(path: *const u8, len: usize) -> SysResult {
    let mut proc = process();
    let path = unsafe { proc.memory_set.check_and_clone_cstr(path)? };
    info!("truncate: path: {:?}, len: {}", path, len);
    proc.lookup_inode(&path)?.resize(len)?;
    Ok(0)
}

pub fn sys_ftruncate(fd: usize, len: usize) -> SysResult {
    info!("ftruncate: fd: {}, len: {}", fd, len);
    process().get_file(fd)?.set_len(len as u64)?;
    Ok(0)
}

pub fn sys_getdents64(fd: usize, buf: *mut LinuxDirent64, buf_size: usize) -> SysResult {
    info!("getdents64: fd: {}, ptr: {:?}, buf_size: {}", fd, buf, buf_size);
    let mut proc = process();
    proc.memory_set.check_mut_array(buf as *mut u8, buf_size)?;
    let file = proc.get_file(fd)?;
    let info = file.metadata()?;
    if info.type_ != FileType::Dir {
        return Err(SysError::ENOTDIR);
    }
    let mut writer = unsafe { DirentBufWriter::new(buf, buf_size) };
    loop {
        let name = match file.read_entry() {
            Err(FsError::EntryNotFound) => break,
            r => r,
        }?;
        // TODO: get ino from dirent
        let ok = writer.try_write(0, 0, &name);
        if !ok { break; }
    }
    Ok(writer.written_size)
}

pub fn sys_dup2(fd1: usize, fd2: usize) -> SysResult {
    info!("dup2: from {} to {}", fd1, fd2);
    let mut proc = process();
    if proc.files.contains_key(&fd2) {
        // close fd2 first if it is opened
        sys_close_internal(&mut proc, fd2)?;
    }

    match proc.files.get(&fd1) {
        Some(FileLike::File(file)) => {
            let new_file = FileLike::File(file.clone());
            proc.files.insert(fd2, new_file);
            Ok(fd2)
        },
        Some(FileLike::Socket(wrapper)) => {
            let new_wrapper = wrapper.clone();
            sys_dup2_socket(&mut proc, new_wrapper, fd2)
        },
        None => Err(SysError::EINVAL)
    }
}

pub fn sys_chdir(path: *const u8) -> SysResult {
    let mut proc = process();
    let path = unsafe { proc.memory_set.check_and_clone_cstr(path)? };
    info!("chdir: path: {:?}", path);

    let inode = proc.lookup_inode(&path)?;
    let info = inode.metadata()?;
    if info.type_ != FileType::Dir {
        return Err(SysError::ENOTDIR);
    }

    if path.len() > 0 && path.as_bytes()[0] == b'/' {
        // absolute
        proc.cwd = path;
    } else {
        // relative
        proc.cwd += &path;
    }
    Ok(0)
}

pub fn sys_rename(oldpath: *const u8, newpath: *const u8) -> SysResult {
    let mut proc = process();
    let oldpath = unsafe { proc.memory_set.check_and_clone_cstr(oldpath)? };
    let newpath = unsafe { proc.memory_set.check_and_clone_cstr(newpath)? };
    info!("rename: oldpath: {:?}, newpath: {:?}", oldpath, newpath);

    let (old_dir_path, old_file_name) = split_path(&oldpath);
    let (new_dir_path, new_file_name) = split_path(&newpath);
    let old_dir_inode = proc.lookup_inode(old_dir_path)?;
    let new_dir_inode = proc.lookup_inode(new_dir_path)?;
    // TODO: merge `rename` and `move` in VFS
    if Arc::ptr_eq(&old_dir_inode, &new_dir_inode) {
        old_dir_inode.rename(old_file_name, new_file_name)?;
    } else {
        old_dir_inode.move_(old_file_name, &new_dir_inode, new_file_name)?;
    }
    Ok(0)
}

pub fn sys_mkdir(path: *const u8, mode: usize) -> SysResult {
    let mut proc = process();
    let path = unsafe { proc.memory_set.check_and_clone_cstr(path)? };
    // TODO: check pathname
    info!("mkdir: path: {:?}, mode: {:#o}", path, mode);

    let (dir_path, file_name) = split_path(&path);
    let inode = proc.lookup_inode(dir_path)?;
    if inode.find(file_name).is_ok() {
        return Err(SysError::EEXIST);
    }
    inode.create(file_name, FileType::Dir, mode as u32)?;
    Ok(0)
}

pub fn sys_link(oldpath: *const u8, newpath: *const u8) -> SysResult {
    let mut proc = process();
    let oldpath = unsafe { proc.memory_set.check_and_clone_cstr(oldpath)? };
    let newpath = unsafe { proc.memory_set.check_and_clone_cstr(newpath)? };
    info!("link: oldpath: {:?}, newpath: {:?}", oldpath, newpath);

    let (new_dir_path, new_file_name) = split_path(&newpath);
    let inode = proc.lookup_inode(&oldpath)?;
    let new_dir_inode = proc.lookup_inode(new_dir_path)?;
    new_dir_inode.link(new_file_name, &inode)?;
    Ok(0)
}

pub fn sys_unlink(path: *const u8) -> SysResult {
    let mut proc = process();
    let path = unsafe { proc.memory_set.check_and_clone_cstr(path)? };
    info!("unlink: path: {:?}", path);

    let (dir_path, file_name) = split_path(&path);
    let dir_inode = proc.lookup_inode(dir_path)?;
    dir_inode.unlink(file_name)?;
    Ok(0)
}

pub fn sys_pipe(fds: *mut u32) -> SysResult {
    info!("pipe: fds: {:?}", fds);

    let mut proc = process();
    proc.memory_set.check_mut_array(fds, 2)?;
    let (read, write) = Pipe::create_pair();
    let read_fd = proc.get_free_inode();

    let read_fd = proc.get_free_inode();
    proc.files.insert(read_fd, FileLike::File(FileHandle::new(Arc::new(read), OpenOptions { read: true, write: false, append: false })));

    let write_fd = proc.get_free_inode();
    proc.files.insert(write_fd, FileLike::File(FileHandle::new(Arc::new(write), OpenOptions { read: false, write: true, append: false })));

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
    fn lookup_inode(&self, path: &str) -> Result<Arc<INode>, SysError> {
        debug!("lookup_inode: cwd {} path {}", self.cwd, path);
        if path.len() > 0 && path.as_bytes()[0] == b'/' {
            // absolute path
            let abs_path = path.split_at(1).1; // skip start '/'
            let inode = ROOT_INODE.lookup(abs_path)?;
            Ok(inode)
        } else {
            // relative path
            let cwd = self.cwd.split_at(1).1; // skip start '/'
            let inode = ROOT_INODE.lookup(cwd)?.lookup(path)?;
            Ok(inode)
        }
    }
}

/// Split a `path` str to `(base_path, file_name)`
fn split_path(path: &str) -> (&str, &str) {
    let mut split = path.trim_end_matches('/').rsplitn(2, '/');
    let file_name = split.next().unwrap();
    let dir_path = split.next().unwrap_or(".");
    (dir_path, file_name)
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
        vm.check_array(iov_ptr, iov_count)?;
        let iovs = unsafe { slice::from_raw_parts(iov_ptr, iov_count) }.to_vec();
        // check all bufs in iov
        for iov in iovs.iter() {
            if iov.len > 0 {
                // skip empty iov
                if readv {
                    vm.check_mut_array(iov.base, iov.len as usize)?;
                } else {
                    vm.check_array(iov.base, iov.len as usize)?;
                }
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
        debug!("copy {:?}", buf);
        for slice in self.0.iter_mut() {
            let copy_len = min(slice.len(), buf.len() - copied_len);
            if copy_len == 0 {
                continue;
            }

            slice[..copy_len].copy_from_slice(&buf[copied_len..copied_len + copy_len]);
            debug!("copy to {:?}", slice);
            copied_len += copy_len;
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

#[repr(C)]
pub struct PollFd {
    fd: u32,
    events: PollEvents,
    revents: PollEvents,
}

bitflags! {
    pub struct PollEvents: u16 {
        /// Nothing Happens
        const NONE = 0x0000;
        /// There is data to read.
        const IN = 0x0001;
        /// Writing is now possible.
        const OUT = 0x0004;
        /// Error condition (return only)
        const ERR = 0x0008;
        /// Hang up (return only)
        const HUP = 0x0010;
        /// Invalid request: fd not open (return only)
        const INVAL = 0x0020;
    }
}
