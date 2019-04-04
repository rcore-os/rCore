use core::fmt;

use super::FileHandle;
use crate::net::Socket;
use crate::syscall::SysResult;
use alloc::boxed::Box;

// TODO: merge FileLike to FileHandle ?
// TODO: fix dup and remove Clone
#[derive(Clone)]
pub enum FileLike {
    File(FileHandle),
    Socket(Box<dyn Socket>),
}

impl FileLike {
    pub fn read(&mut self, buf: &mut [u8]) -> SysResult {
        let len = match self {
            FileLike::File(file) => file.read(buf)?,
            FileLike::Socket(socket) => socket.read(buf).0?,
        };
        Ok(len)
    }
    pub fn write(&mut self, buf: &[u8]) -> SysResult {
        let len = match self {
            FileLike::File(file) => file.write(buf)?,
            FileLike::Socket(socket) => socket.write(buf, None)?,
        };
        Ok(len)
    }
    pub fn ioctl(&mut self, request: usize, arg1: usize, arg2: usize, arg3: usize) -> SysResult {
        match self {
            FileLike::File(file) => {
                warn!("ioctl not implemented for file");
                Ok(0)
            }
            FileLike::Socket(socket) => socket.ioctl(request, arg1, arg2, arg3),
        }
    }
}

impl fmt::Debug for FileLike {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FileLike::File(_) => write!(f, "File"),
            FileLike::Socket(_) => write!(f, "Socket"),
        }
    }
}
