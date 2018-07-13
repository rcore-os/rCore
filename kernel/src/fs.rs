use simple_filesystem::*;
use alloc::boxed::Box;
use arch::driver::ide;
use spin::Mutex;
use process;

#[cfg(not(feature = "link_user_program"))]
pub fn load_sfs() {
//    let slice = unsafe { MemBuf::new(_binary_user_ucore32_img_start, _binary_user_ucore32_img_end) };
    let sfs = SimpleFileSystem::open(Box::new(&ide::DISK0)).unwrap();
    let root = sfs.root_inode();
    let files = root.borrow().list().unwrap();
    trace!("Loading programs: {:?}", files);

//    for name in files.iter().filter(|&f| f != "." && f != "..") {
    for name in files.iter().filter(|&f| f == "sleep") {
        static mut BUF: [u8; 64 << 12] = [0; 64 << 12];
        let file = root.borrow().lookup(name.as_str()).unwrap();
        let len = file.borrow().read_at(0, unsafe { &mut BUF }).unwrap();
        process::add_user_process(name, unsafe { &BUF[..len] });
    }

    process::print();
}

#[cfg(feature = "link_user_program")]
pub fn load_sfs() {
    let slice = unsafe {
        slice::from_raw_parts(_binary_hello_start as *const u8,
                              _binary_hello_size as usize)
    };

    process::add_user_process("hello", slice);
    process::print();
}


#[cfg(feature = "link_user_program")]
extern {
    fn _binary_hello_start();
    fn _binary_hello_size();
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