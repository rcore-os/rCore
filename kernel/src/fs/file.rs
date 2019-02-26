//! File handle for process

use alloc::{string::String, sync::Arc};

use rcore_fs::vfs::{Metadata, INode, Result, FsError};

#[derive(Clone)]
pub struct FileHandle {
    inode: Arc<INode>,
    offset: u64,
    options: OpenOptions,
}

#[derive(Debug, Clone)]
pub struct OpenOptions {
    pub read: bool,
    pub write: bool,
    /// Before each write, the file offset is positioned at the end of the file.
    pub append: bool,
}

#[derive(Debug)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

impl FileHandle {
    pub fn new(inode: Arc<INode>, options: OpenOptions) -> Self {
        FileHandle {
            inode,
            offset: 0,
            options,
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if !self.options.read {
            return Err(FsError::InvalidParam);  // FIXME: => EBADF
        }
        let len = self.inode.read_at(self.offset as usize, buf)?;
        self.offset += len as u64;
        Ok(len)
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize> {
        if !self.options.write {
            return Err(FsError::InvalidParam);  // FIXME: => EBADF
        }
        if self.options.append {
            let info = self.inode.metadata()?;
            self.offset = info.size as u64;
        }
        let len = self.inode.write_at(self.offset as usize, buf)?;
        self.offset += len as u64;
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

    pub fn info(&self) -> Result<Metadata> {
        self.inode.metadata()
    }

    pub fn get_entry(&self, id: usize) -> Result<String> {
        self.inode.get_entry(id)
    }
}