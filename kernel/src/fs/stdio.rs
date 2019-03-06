//! Implement INode for Stdin & Stdout

use alloc::{collections::vec_deque::VecDeque, string::String, sync::Arc};
use core::any::Any;

use rcore_fs::vfs::*;

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
        // QEMU v3.0 don't support M-mode external interrupt (bug?)
        // So we have to use polling.
        #[cfg(feature = "m_mode")]
            loop {
            let c = crate::arch::io::getchar();
            if c != '\0' { return c; }
        }
        #[cfg(not(feature = "m_mode"))]
            loop {
            let ret = self.buf.lock().pop_front();
            match ret {
                Some(c) => return c,
                None => self.pushed._wait(),
            }
        }
    }
    pub fn can_read(&self) -> bool {
        self.buf.lock().len() > 0
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
        fn sync(&self) -> Result<()> { Ok(()) }
        fn resize(&self, _len: usize) -> Result<()> { Err(FsError::NotSupported) }
        fn create(&self, _name: &str, _type_: FileType, _mode: u32) -> Result<Arc<INode>> { Err(FsError::NotDir) }
        fn unlink(&self, _name: &str) -> Result<()> { Err(FsError::NotDir) }
        fn link(&self, _name: &str, _other: &Arc<INode>) -> Result<()> { Err(FsError::NotDir) }
        fn rename(&self, _old_name: &str, _new_name: &str) -> Result<()> { Err(FsError::NotDir) }
        fn move_(&self, _old_name: &str, _target: &Arc<INode>, _new_name: &str) -> Result<()> { Err(FsError::NotDir) }
        fn find(&self, _name: &str) -> Result<Arc<INode>> { Err(FsError::NotDir) }
        fn get_entry(&self, _id: usize) -> Result<String> { Err(FsError::NotDir) }
        fn fs(&self) -> Arc<FileSystem> { unimplemented!() }
        fn as_any_ref(&self) -> &Any { self }
    };
}

impl INode for Stdin {
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> Result<usize> {
        buf[0] = self.pop() as u8;
        Ok(1)
    }
    fn write_at(&self, _offset: usize, _buf: &[u8]) -> Result<usize> { unimplemented!() }
    impl_inode!();
}

impl INode for Stdout {
    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> Result<usize> { unimplemented!() }
    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        use core::str;
        //we do not care the utf-8 things, we just want to print it!
        let s = unsafe{ str::from_utf8_unchecked(buf) };
        print!("{}", s);
        Ok(buf.len())
    }
    impl_inode!();
}