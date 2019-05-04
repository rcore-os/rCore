//! Implement INode for Stdin & Stdout

use alloc::{collections::vec_deque::VecDeque, string::String, sync::Arc};
use core::any::Any;

use rcore_fs::vfs::*;

use super::ioctl::*;
use crate::sync::Condvar;
use crate::sync::SpinNoIrqLock as Mutex;

#[derive(Default)]
pub struct Stdin {
    buf: Mutex<VecDeque<char>>,
    pub pushed: Condvar,
}

impl Stdin {
    pub fn push(&self, c: char) {
        self.buf.lock().push_back(c);
        self.pushed.notify_one();
    }
    pub fn pop(&self) -> char {
        #[cfg(feature = "board_k210")]
        loop {
            // polling
            let c = crate::arch::io::getchar();
            if c != '\0' {
                return c;
            }
        }
        #[cfg(feature = "board_rocket_chip")]
        loop {
            let c = crate::arch::io::getchar();
            if c != '\0' && c as u8 != 254 {
                return c;
            }
        }
        #[cfg(not(any(feature = "board_k210", feature = "board_rocket_chip")))]
        loop {
            let mut buf_lock = self.buf.lock();
            match buf_lock.pop_front() {
                Some(c) => return c,
                None => {
                    self.pushed.wait(buf_lock);
                }
            }
        }
    }
    pub fn can_read(&self) -> bool {
        // Currently, rocket-chip implementation rely on htif interface, the serial interrupt DO
        // NOT work, so return true always
        #[cfg(feature = "board_rocket_chip")]
        return true;
        #[cfg(not(feature = "board_rocket_chip"))]
        return self.buf.lock().len() > 0;
    }
}

#[derive(Default)]
pub struct Stdout;

lazy_static! {
    pub static ref STDIN: Arc<Stdin> = Arc::new(Stdin::default());
    pub static ref STDOUT: Arc<Stdout> = Arc::new(Stdout::default());
}

// TODO: better way to provide default impl?
macro_rules! impl_inode {
    () => {
        fn metadata(&self) -> Result<Metadata> { Err(FsError::NotSupported) }
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
        fn io_control(&self, cmd: u32, data: usize) -> Result<()> {
            match cmd as usize {
                TCGETS | TIOCGWINSZ | TIOCSPGRP => {
                    // pretend to be tty
                    Ok(())
                },
                TIOCGPGRP => {
                    // pretend to be have a tty process group
                    // TODO: verify pointer
                    unsafe {
                        *(data as *mut u32) = 0
                    };
                    Ok(())
                }
                _ => Err(FsError::NotSupported)
            }
        }
        fn fs(&self) -> Arc<FileSystem> { unimplemented!() }
        fn as_any_ref(&self) -> &Any { self }
    };
}

impl INode for Stdin {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        buf[0] = self.pop() as u8;
        Ok(1)
    }
    fn write_at(&self, _offset: usize, _buf: &[u8]) -> Result<usize> {
        unimplemented!()
    }
    fn poll(&self) -> Result<PollStatus> {
        Ok(PollStatus {
            read: self.can_read(),
            write: false,
            error: false,
        })
    }
    impl_inode!();
}

impl INode for Stdout {
    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> Result<usize> {
        unimplemented!()
    }
    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        use core::str;
        //we do not care the utf-8 things, we just want to print it!
        let s = unsafe { str::from_utf8_unchecked(buf) };
        print!("{}", s);
        Ok(buf.len())
    }
    fn poll(&self) -> Result<PollStatus> {
        Ok(PollStatus {
            read: false,
            write: true,
            error: false,
        })
    }
    impl_inode!();
}
