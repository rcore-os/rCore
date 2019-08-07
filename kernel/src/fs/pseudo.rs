//! Pseudo file system INode

use alloc::{string::String, sync::Arc, vec::Vec};
use core::any::Any;

use rcore_fs::vfs::*;

pub struct Pseudo {
    content: Vec<u8>,
    type_: FileType,
}

impl Pseudo {
    pub fn new(s: &str, type_: FileType) -> Self {
        Pseudo {
            content: Vec::from(s.as_bytes()),
            type_,
        }
    }
}

impl INode for Pseudo {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        if offset >= self.content.len() {
            return Ok(0);
        }
        let len = (self.content.len() - offset).min(buf.len());
        buf[..len].copy_from_slice(&self.content[offset..offset + len]);
        Ok(len)
    }
    fn write_at(&self, _offset: usize, _buf: &[u8]) -> Result<usize> {
        Err(FsError::NotSupported)
    }
    fn poll(&self) -> Result<PollStatus> {
        Ok(PollStatus {
            read: true,
            write: false,
            error: false,
        })
    }
    fn metadata(&self) -> Result<Metadata> {
        Ok(Metadata {
            dev: 0,
            inode: 0,
            size: self.content.len(),
            blk_size: 0,
            blocks: 0,
            atime: Timespec { sec: 0, nsec: 0 },
            mtime: Timespec { sec: 0, nsec: 0 },
            ctime: Timespec { sec: 0, nsec: 0 },
            type_: self.type_,
            mode: 0,
            nlinks: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
        })
    }
    fn as_any_ref(&self) -> &dyn Any {
        self
    }
}
