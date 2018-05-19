use simple_filesystem::*;
use alloc::boxed::Box;
use process;

extern {
    fn _binary_user_sfs_img_start();
    fn _binary_user_sfs_img_end();
    fn _binary_user_forktest_start();
    fn _binary_user_forktest_end();
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

pub fn load_sfs() {
    let slice = unsafe { MemBuf::new(_binary_user_sfs_img_start, _binary_user_sfs_img_end) };
    let sfs = SimpleFileSystem::open(Box::new(slice)).unwrap();
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

//    process::add_user_process("forktest", unsafe { MemBuf::new(_binary_user_forktest_start, _binary_user_forktest_end).0 });

    process::print();
}