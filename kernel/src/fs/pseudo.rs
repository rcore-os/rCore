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

// TODO: better way to provide default impl?
macro_rules! impl_inode {
    () => {
        fn set_metadata(&self, _metadata: &Metadata) -> Result<()> { Ok(()) }
        fn sync_all(&self) -> Result<()> { Ok(()) }
        fn sync_data(&self) -> Result<()> { Ok(()) }
        fn resize(&self, _len: usize) -> Result<()> { Err(FsError::NotSupported) }
        fn create(&self, _name: &str, _type_: FileType, _mode: u32) -> Result<Arc<INode>> { Err(FsError::NotDir) }
        fn unlink(&self, _name: &str) -> Result<()> { Err(FsError::NotDir) }
        fn link(&self, _name: &str, _other: &Arc<INode>) -> Result<()> { Err(FsError::NotDir) }
        fn move_(&self, _old_name: &str, _target: &Arc<INode>, _new_name: &str) -> Result<()> { Err(FsError::NotDir) }
        fn find(&self, _name: &str) -> Result<Arc<INode>> { Err(FsError::NotDir) }
        fn get_entry(&self, _id: usize) -> Result<String> { Err(FsError::NotDir) }
        fn io_control(&self, cmd: u32, data: usize) -> Result<()> { Err(FsError::NotSupported) }
        fn fs(&self) -> Arc<FileSystem> { unimplemented!() }
        fn as_any_ref(&self) -> &Any { self }
    };
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
        })
    }
    impl_inode!();
}
