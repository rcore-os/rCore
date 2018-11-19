use simple_filesystem::*;
use alloc::{boxed::Box, sync::Arc, string::String, collections::VecDeque, vec::Vec};
use core::any::Any;
use core::slice;
use lazy_static::lazy_static;
use crate::memory::{MemorySet, InactivePageTable0, memory_set_record};
use crate::process::context::memory_set_map_swappable;
#[cfg(target_arch = "x86_64")]
use crate::arch::driver::ide;
use crate::sync::Condvar;
use crate::sync::SpinNoIrqLock as Mutex;

// Hard link user program
#[cfg(target_arch = "riscv32")]
global_asm!(r#"
    .section .rodata
    .align 12
    .global _user_img_start
    .global _user_img_end
_user_img_start:
    .incbin "../user/user-riscv.img"
_user_img_end:
"#);

lazy_static! {
    pub static ref ROOT_INODE: Arc<INode> = {
        #[cfg(target_arch = "riscv32")]
        let device = {
            extern {
                fn _user_img_start();
                fn _user_img_end();
            }
            Box::new(unsafe { MemBuf::new(_user_img_start, _user_img_end) })
        };
        #[cfg(target_arch = "x86_64")]
        let device = Box::new(ide::IDE::new(1));
        #[cfg(target_arch = "aarch64")]
        let device = unimplemented!();

        let sfs = SimpleFileSystem::open(device).expect("failed to open SFS");
        sfs.root_inode()
    };
}

struct MemBuf(&'static [u8]);

impl MemBuf {
    unsafe fn new(begin: unsafe extern fn(), end: unsafe extern fn()) -> Self {
        use core::slice;
        MemBuf(slice::from_raw_parts(begin as *const u8, end as usize - begin as usize))
    }
}

impl Device for MemBuf {
    fn read_at(&mut self, offset: usize, buf: &mut [u8]) -> Option<usize> {
        let slice = self.0;
        let len = buf.len().min(slice.len() - offset);
        buf[..len].copy_from_slice(&slice[offset..offset + len]);
        Some(len)
    }
    fn write_at(&mut self, offset: usize, buf: &[u8]) -> Option<usize> {
        None
    }
}

#[cfg(target_arch = "x86_64")]
impl BlockedDevice for ide::IDE {
    const BLOCK_SIZE_LOG2: u8 = 9;
    fn read_at(&mut self, block_id: usize, buf: &mut [u8]) -> bool {
        assert!(buf.len() >= ide::BLOCK_SIZE);
        let buf = unsafe { slice::from_raw_parts_mut(buf.as_ptr() as *mut u32, ide::BLOCK_SIZE / 4) };
        self.read(block_id as u64, 1, buf).is_ok()
    }
    fn write_at(&mut self, block_id: usize, buf: &[u8]) -> bool {
        assert!(buf.len() >= ide::BLOCK_SIZE);
        let buf = unsafe { slice::from_raw_parts(buf.as_ptr() as *mut u32, ide::BLOCK_SIZE / 4) };
        self.write(block_id as u64, 1, buf).is_ok()
    }
}

#[derive(Default)]
pub struct Stdin {
    buf: Mutex<VecDeque<char>>,
    pushed: Condvar,
}

impl Stdin {
    pub fn push(&self, c: char) {
        self.buf.lock().push_back(c);
        self.pushed.notify_one();
    }
    pub fn pop(&self) -> char {
        loop {
            let ret = self.buf.lock().pop_front();
            match ret {
                Some(c) => return c,
                None => self.pushed._wait(),
            }
        }
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
        fn info(&self) -> Result<FileInfo> { unimplemented!() }
        fn sync(&self) -> Result<()> { unimplemented!() }
        fn resize(&self, len: usize) -> Result<()> { unimplemented!() }
        fn create(&self, name: &str, type_: FileType) -> Result<Arc<INode>> { unimplemented!() }
        fn unlink(&self, name: &str) -> Result<()> { unimplemented!() }
        fn link(&self, name: &str, other: &Arc<INode>) -> Result<()> { unimplemented!() }
        fn rename(&self, old_name: &str, new_name: &str) -> Result<()> { unimplemented!() }
        fn move_(&self, old_name: &str, target: &Arc<INode>, new_name: &str) -> Result<()> { unimplemented!() }
        fn find(&self, name: &str) -> Result<Arc<INode>> { unimplemented!() }
        fn get_entry(&self, id: usize) -> Result<String> { unimplemented!() }
        fn fs(&self) -> Arc<FileSystem> { unimplemented!() }
        fn as_any_ref(&self) -> &Any { self }
    };
}

impl INode for Stdin {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        buf[0] = self.pop() as u8;
        Ok(1)
    }
    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> { unimplemented!() }
    impl_inode!();
}

impl INode for Stdout {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> { unimplemented!() }
    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        use core::str;
        let s = str::from_utf8(buf).map_err(|_| ())?;
        print!("{}", s);
        Ok(buf.len())
    }
    impl_inode!();
}

pub trait INodeExt {
    fn read_as_vec(&self) -> Result<Vec<u8>>;
}

impl INodeExt for INode {
    fn read_as_vec(&self) -> Result<Vec<u8>> {
        let size = self.info()?.size;
        let mut buf = Vec::with_capacity(size);
        unsafe { buf.set_len(size); }
        self.read_at(0, buf.as_mut_slice())?;
        Ok(buf)
    }
}
