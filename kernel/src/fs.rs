use simple_filesystem::*;
use alloc::boxed::Box;
#[cfg(target_arch = "x86_64")]
use arch::driver::ide;
use spin::Mutex;
use process;

pub fn load_sfs() {
    #[cfg(target_arch = "riscv")]
    let device = {
        extern {
            fn _binary_user_riscv_img_start();
            fn _binary_user_riscv_img_end();
        }
        Box::new(unsafe { MemBuf::new(_binary_user_riscv_img_start, _binary_user_riscv_img_end) })
    };
    #[cfg(target_arch = "x86_64")]
    let device = Box::new(&ide::DISK0);
    let sfs = SimpleFileSystem::open(device).unwrap();
    let root = sfs.root_inode();
    let files = root.borrow().list().unwrap();
    trace!("Loading programs: {:?}", files);

//    for name in files.iter().filter(|&f| f != "." && f != "..") {
    for name in files.iter().filter(|&f| f == "hello") {
        static mut BUF: [u8; 64 << 12] = [0; 64 << 12];
        let file = root.borrow().lookup(name.as_str()).unwrap();
        let len = file.borrow().read_at(0, unsafe { &mut BUF }).unwrap();
        process::add_user_process(name, unsafe { &BUF[..len] });
    }

    process::print();
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
impl BlockedDevice for &'static ide::DISK0 {
    fn block_size_log2(&self) -> u8 {
        debug_assert_eq!(ide::BLOCK_SIZE, 512);
        9
    }
    fn read_at(&mut self, block_id: usize, buf: &mut [u8]) -> bool {
        assert!(buf.len() >= ide::BLOCK_SIZE);
        let buf = unsafe { slice::from_raw_parts_mut(buf.as_ptr() as *mut u32, ide::BLOCK_SIZE / 4) };
        self.0.lock().read(block_id as u64, 1, buf).is_ok()
    }
    fn write_at(&mut self, block_id: usize, buf: &[u8]) -> bool {
        assert!(buf.len() >= ide::BLOCK_SIZE);
        let buf = unsafe { slice::from_raw_parts(buf.as_ptr() as *mut u32, ide::BLOCK_SIZE / 4) };
        self.0.lock().write(block_id as u64, 1, buf).is_ok()
    }
}