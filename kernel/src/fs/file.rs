//! File handle for process

use crate::memory::GlobalFrameAlloc;
use crate::process::{current_thread, INodeForMap};
use crate::syscall::{MmapProt, SysResult, TimeSpec};
use crate::{processor, thread};
use alloc::{string::String, sync::Arc};
use core::fmt;

use rcore_fs::vfs::FsError::NotSupported;
use rcore_fs::vfs::{FileType, FsError, INode, MMapArea, Metadata, PollStatus, Result};
use rcore_memory::memory_set::handler::File;

use crate::fs::fcntl::{O_APPEND, O_NONBLOCK};
use crate::sync::SpinLock as Mutex;
use crate::syscall::SysError::{EAGAIN, ESPIPE};
use bitflags::_core::cell::Cell;
use spin::RwLock;

enum Flock {
    None = 0,
    Shared = 1,
    Exclusive = 2,
}

struct OpenFileDescription {
    offset: u64,
    options: OpenOptions,
    flock: Flock,
}

impl OpenFileDescription {
    fn create(options: OpenOptions) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(OpenFileDescription {
            offset: 0,
            options,
            flock: Flock::None,
        }))
    }
}

#[derive(Clone)]
pub struct FileHandle {
    inode: Arc<dyn INode>,
    description: Arc<RwLock<OpenFileDescription>>,
    pub path: String,
    pub pipe: bool, // specify if this is pipe, socket, or FIFO
    pub fd_cloexec: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct OpenOptions {
    pub read: bool,
    pub write: bool,
    /// Before each write, the file offset is positioned at the end of the file.
    pub append: bool,
    pub nonblock: bool,
}

#[derive(Debug)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

impl FileHandle {
    pub fn new(
        inode: Arc<dyn INode>,
        options: OpenOptions,
        path: String,
        pipe: bool,
        fd_cloexec: bool,
    ) -> Self {
        return FileHandle {
            inode,
            description: OpenFileDescription::create(options),
            path,
            pipe,
            fd_cloexec,
        };
    }

    // do almost as default clone does, but with fd_cloexec specified
    pub fn dup(&self, fd_cloexec: bool) -> Self {
        FileHandle {
            inode: self.inode.clone(),
            description: self.description.clone(),
            path: self.path.clone(),
            pipe: self.pipe,
            fd_cloexec, // this field do not share
        }
    }

    pub fn set_options(&self, arg: usize) {
        let options = &mut self.description.write().options;
        options.nonblock = (arg & O_NONBLOCK) != 0;
        // TODO: handle append
        // options.append = (arg & O_APPEND) != 0;
    }

    // pub fn get_options(&self) -> usize {
    // let options = self.description.read().options;
    // let mut ret = 0 as usize;
    // }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let len = self.read_at(self.description.read().offset as usize, buf)?;
        self.description.write().offset += len as u64;
        Ok(len)
    }

    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        let options = &self.description.read().options;
        if !options.read {
            return Err(FsError::InvalidParam); // FIXME: => EBADF
        }
        if !options.nonblock {
            // block
            loop {
                match self.inode.read_at(offset, buf) {
                    Ok(read_len) => {
                        return Ok(read_len);
                    }
                    Err(FsError::Again) => {
                        thread::yield_now();
                    }
                    Err(err) => {
                        return Err(err);
                    }
                }
            }
        } else {
            let len = self.inode.read_at(offset, buf)?;
            Ok(len)
        }
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let description = self.description.read();
        let offset = match description.options.append {
            true => self.inode.metadata()?.size as u64,
            false => description.offset,
        } as usize;
        drop(description);
        let len = self.write_at(offset, buf)?;
        self.description.write().offset += len as u64;
        Ok(len)
    }

    pub fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        if !self.description.read().options.write {
            return Err(FsError::InvalidParam); // FIXME: => EBADF
        }
        let len = self.inode.write_at(offset, buf)?;
        TimeSpec::update(&self.inode);
        Ok(len)
    }

    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let mut description = self.description.write();
        description.offset = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => (self.inode.metadata()?.size as i64 + offset) as u64,
            SeekFrom::Current(offset) => (description.offset as i64 + offset) as u64,
        };
        Ok(description.offset)
    }

    pub fn set_len(&mut self, len: u64) -> Result<()> {
        if !self.description.read().options.write {
            return Err(FsError::InvalidParam); // FIXME: => EBADF
        }
        self.inode.resize(len as usize)?;
        Ok(())
    }

    pub fn sync_all(&mut self) -> Result<()> {
        self.inode.sync_all()
    }

    pub fn sync_data(&mut self) -> Result<()> {
        self.inode.sync_data()
    }

    pub fn metadata(&self) -> Result<Metadata> {
        self.inode.metadata()
    }

    pub fn lookup_follow(&self, path: &str, max_follow: usize) -> Result<Arc<dyn INode>> {
        self.inode.lookup_follow(path, max_follow)
    }

    pub fn read_entry(&mut self) -> Result<String> {
        let mut description = self.description.write();
        if !description.options.read {
            return Err(FsError::InvalidParam); // FIXME: => EBADF
        }
        let mut offset = &mut description.offset;
        let name = self.inode.get_entry(*offset as usize)?;
        *offset += 1;
        Ok(name)
    }

    pub fn read_entry_with_metadata(&mut self) -> Result<(Metadata, String)> {
        let mut description = self.description.write();
        if !description.options.read {
            return Err(FsError::InvalidParam); // FIXME: => EBADF
        }
        let mut offset = &mut description.offset;
        let ret = self.inode.get_entry_with_metadata(*offset as usize)?;
        *offset += 1;
        Ok(ret)
    }

    pub fn poll(&self) -> Result<PollStatus> {
        self.inode.poll()
    }

    pub fn io_control(&self, cmd: u32, arg: usize) -> Result<usize> {
        self.inode.io_control(cmd, arg)
    }

    pub fn mmap(&mut self, area: MMapArea) -> Result<()> {
        info!("mmap file path is {}", self.path);
        match self.inode.metadata()?.type_ {
            FileType::File => {
                let prot = MmapProt::from_bits_truncate(area.prot);
                let thread = unsafe { current_thread() };
                thread.vm.lock().push(
                    area.start_vaddr,
                    area.end_vaddr,
                    prot.to_attr(),
                    File {
                        file: INodeForMap(self.inode.clone()),
                        mem_start: area.start_vaddr,
                        file_start: area.offset,
                        file_end: area.offset + area.end_vaddr - area.start_vaddr,
                        allocator: GlobalFrameAlloc,
                    },
                    "mmap_file",
                );
                Ok(())
            }
            FileType::CharDevice => self.inode.mmap(area),
            _ => Err(FsError::NotSupported),
        }
    }

    pub fn inode(&self) -> Arc<dyn INode> {
        self.inode.clone()
    }
}

impl fmt::Debug for FileHandle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = self.description.read();
        return f
            .debug_struct("FileHandle")
            .field("offset", &description.offset)
            .field("options", &description.options)
            .field("path", &self.path)
            .finish();
    }
}
