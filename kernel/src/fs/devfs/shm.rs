use core::any::Any;
use rcore_fs::vfs::*;

pub use super::{STDIN, STDOUT};
use rcore_fs::vfs::FsError::NotSupported;

// try to create directory under /dev
// do not have enough time to come up with a better way.

#[derive(Default)]
pub struct ShmINode;

impl INode for ShmINode {
    /// Read bytes at `offset` into `buf`, return the number of bytes read.
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        Err(NotSupported)
    }

    /// Write bytes at `offset` from `buf`, return the number of bytes written.
    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        Err(NotSupported)
    }

    /// Poll the events, return a bitmap of events.
    fn poll(&self) -> Result<PollStatus> {
        Ok(PollStatus {
            read: false,
            write: false,
            error: false,
        })
    }

    /// Get metadata of the INode
    fn metadata(&self) -> Result<Metadata> {
        Ok(Metadata {
            dev: 1,
            inode: 2,
            size: 0,
            blk_size: 0,
            blocks: 0,
            atime: Timespec { sec: 0, nsec: 0 },
            mtime: Timespec { sec: 0, nsec: 0 },
            ctime: Timespec { sec: 0, nsec: 0 },
            type_: FileType::Dir,
            mode: 0o666,
            nlinks: 1,
            uid: 0,
            gid: 0,
            rdev: make_rdev(0, 40),
        })
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }
}
