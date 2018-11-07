use simple_filesystem::*;
use alloc::{boxed::Box, sync::Arc};
#[cfg(target_arch = "x86_64")]
use arch::driver::ide;
use spin::Mutex;

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

        let sfs = SimpleFileSystem::open(device).expect("failed to open SFS");
        sfs.root_inode()
    };
}

pub fn shell() {
    let files = ROOT_INODE.list().unwrap();
    println!("Available programs: {:?}", files);

    // Avoid stack overflow in release mode
    // Equal to: `buf = Box::new([0; 64 << 12])`
    use alloc::alloc::{alloc, dealloc, Layout};
    const BUF_SIZE: usize = 0x40000;
    let layout = Layout::from_size_align(BUF_SIZE, 0x1000).unwrap();
    let buf = unsafe{ slice::from_raw_parts_mut(alloc(layout), BUF_SIZE) };
    loop {
        print!(">> ");
        use console::get_line;
        let cmd = get_line();
        if cmd == "" {
            continue;
        }
        let name = cmd.split(' ').next().unwrap();
        if let Ok(file) = ROOT_INODE.lookup(name) {
            use process::*;
            let len = file.read_at(0, &mut *buf).unwrap();
            let pid = processor().manager().add(ContextImpl::new_user(&buf[..len], cmd.as_str()));
            processor().manager().wait(thread::current().id(), pid);
            processor().yield_now();
        } else {
            println!("Program not exist");
        }
    }
    unsafe { dealloc(buf.as_mut_ptr(), layout) };
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

use core::slice;

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