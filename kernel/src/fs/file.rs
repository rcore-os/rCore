//! File handle for process

use crate::memory::GlobalFrameAlloc;
use crate::process::{current_thread, INodeForMap};
use crate::syscall::MmapProt;
use crate::thread;
use alloc::{string::String, sync::Arc};
use core::fmt;

use rcore_fs::vfs::{FileType, FsError, INode, MMapArea, Metadata, PollStatus, Result};
use rcore_memory::memory_set::handler::File;

#[derive(Clone)]
pub struct FileHandle {
    inode: Arc<dyn INode>,
    offset: u64,
    options: OpenOptions,
    pub path: String,
}

#[derive(Debug, Clone)]
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
    pub fn new(inode: Arc<dyn INode>, options: OpenOptions, path: String) -> Self {
        return FileHandle {
            inode,
            offset: 0,
            options,
            path,
        };
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let len = self.read_at(self.offset as usize, buf)?;
        self.offset += len as u64;
        Ok(len)
    }

    pub fn read_at(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        if !self.options.read {
            return Err(FsError::InvalidParam); // FIXME: => EBADF
        }
        if !self.options.nonblock {
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
        let offset = match self.options.append {
            true => self.inode.metadata()?.size as u64,
            false => self.offset,
        } as usize;
        let len = self.write_at(offset, buf)?;
        self.offset = (offset + len) as u64;
        Ok(len)
    }

    pub fn write_at(&mut self, offset: usize, buf: &[u8]) -> Result<usize> {
        if !self.options.write {
            return Err(FsError::InvalidParam); // FIXME: => EBADF
        }
        let len = self.inode.write_at(offset, buf)?;
        Ok(len)
    }

    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.offset = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => (self.inode.metadata()?.size as i64 + offset) as u64,
            SeekFrom::Current(offset) => (self.offset as i64 + offset) as u64,
        };
        Ok(self.offset)
    }

    pub fn set_len(&mut self, len: u64) -> Result<()> {
        if !self.options.write {
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
        if !self.options.read {
            return Err(FsError::InvalidParam); // FIXME: => EBADF
        }
        let name = self.inode.get_entry(self.offset as usize)?;
        self.offset += 1;
        Ok(name)
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

    pub fn fcntl(&mut self, cmd: usize, arg: usize) -> Result<()> {
        if arg & 0x800 > 0 && cmd == 4 {
            self.options.nonblock = true;
        }
        Ok(())
    }
}

impl fmt::Debug for FileHandle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return f
            .debug_struct("FileHandle")
            .field("offset", &self.offset)
            .field("options", &self.options)
            .field("path", &self.path)
            .finish();
    }
}
