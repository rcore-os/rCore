//! File structure for process

use alloc::{string::String, sync::Arc};

use simple_filesystem::{FileInfo, INode, Result};

#[derive(Clone)]
pub struct File {
    inode: Arc<INode>,
    offset: usize,
    readable: bool,
    writable: bool,
}

impl File {
    pub fn new(inode: Arc<INode>, readable: bool, writable: bool) -> Self {
        File { inode, offset: 0, readable, writable }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        assert!(self.readable);
        let len = self.inode.read_at(self.offset, buf)?;
        self.offset += len;
        Ok(len)
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize> {
        assert!(self.writable);
        let len = self.inode.write_at(self.offset, buf)?;
        self.offset += len;
        Ok(len)
    }

    pub fn info(&self) -> Result<FileInfo> {
        self.inode.info()
    }

    pub fn get_entry(&self, id: usize) -> Result<String> {
        self.inode.get_entry(id)
    }
}