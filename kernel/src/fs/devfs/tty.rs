use core::any::Any;
use rcore_fs::vfs::*;

pub use super::{STDIN, STDOUT};
use crate::fs::ioctl::*;
use crate::process::{current_thread, Pgid};
use crate::syscall::SysError;
use alloc::sync::Arc;
use rcore_fs::vfs::FsError::NotSupported;
use spin::RwLock;

// Ref: [https://linux.die.net/man/4/tty]
#[derive(Default)]
pub struct TtyINode {
    pub foreground_pgid: RwLock<Pgid>,
}

lazy_static! {
    pub static ref TTY: Arc<TtyINode> = Arc::new(TtyINode::default());
}

pub fn foreground_pgid() -> Pgid {
    *TTY.foreground_pgid.read()
}

impl INode for TtyINode {
    /// Read bytes at `offset` into `buf`, return the number of bytes read.
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        STDIN.read_at(offset, buf)
    }

    /// Write bytes at `offset` from `buf`, return the number of bytes written.
    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        STDOUT.write_at(offset, buf)
    }

    /// Poll the events, return a bitmap of events.
    fn poll(&self) -> Result<PollStatus> {
        Ok(PollStatus {
            read: STDIN.can_read(),
            write: true,
            error: false,
        })
    }

    fn io_control(&self, cmd: u32, data: usize) -> Result<usize> {
        let cmd = cmd as usize;
        match cmd {
            TIOCGPGRP => {
                // TODO: check the pointer?
                let argp = data as *mut i32; // pid_t
                unsafe { *argp = *self.foreground_pgid.read() };
                Ok(0)
            }
            TIOCSPGRP => {
                let fpgid = unsafe { *(data as *const i32) };
                *self.foreground_pgid.write() = fpgid;
                info!("tty: set foreground process group to {}", fpgid);
                Ok(0)
            }
            _ => Err(NotSupported),
        }
    }

    /// Get metadata of the INode
    fn metadata(&self) -> Result<Metadata> {
        Ok(Metadata {
            dev: 1,
            inode: 13,
            size: 0,
            blk_size: 0,
            blocks: 0,
            atime: Timespec { sec: 0, nsec: 0 },
            mtime: Timespec { sec: 0, nsec: 0 },
            ctime: Timespec { sec: 0, nsec: 0 },
            type_: FileType::CharDevice,
            mode: 0o666,
            nlinks: 1,
            uid: 0,
            gid: 0,
            rdev: make_rdev(5, 0),
        })
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }
}
