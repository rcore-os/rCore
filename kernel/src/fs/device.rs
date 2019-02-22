//! Implement Device

use spin::RwLock;
use rcore_fs::dev::*;

#[cfg(target_arch = "x86_64")]
use crate::arch::driver::ide;

#[cfg(not(target_arch = "x86_64"))]
pub struct MemBuf(RwLock<&'static mut [u8]>);

#[cfg(not(target_arch = "x86_64"))]
impl MemBuf {
    unsafe fn new(begin: unsafe extern fn(), end: unsafe extern fn()) -> Self {
        use core::slice;
        MemBuf(RwLock::new(slice::from_raw_parts_mut(begin as *mut u8, end as usize - begin as usize)))
    }
}

#[cfg(not(target_arch = "x86_64"))]
impl Device for MemBuf {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Option<usize> {
        let slice = self.0.read();
        let len = buf.len().min(slice.len() - offset);
        buf[..len].copy_from_slice(&slice[offset..offset + len]);
        Some(len)
    }
    fn write_at(&self, offset: usize, buf: &[u8]) -> Option<usize> {
        let mut slice = self.0.write();
        let len = buf.len().min(slice.len() - offset);
        slice[offset..offset + len].copy_from_slice(&buf[..len]);
        Some(len)
    }
}

#[cfg(target_arch = "x86_64")]
impl BlockDevice for ide::IDE {
    const BLOCK_SIZE_LOG2: u8 = 9;
    fn read_at(&self, block_id: usize, buf: &mut [u8]) -> bool {
        use core::slice;
        assert!(buf.len() >= ide::BLOCK_SIZE);
        let buf = unsafe { slice::from_raw_parts_mut(buf.as_ptr() as *mut u32, ide::BLOCK_SIZE / 4) };
        self.read(block_id as u64, 1, buf).is_ok()
    }
    fn write_at(&self, block_id: usize, buf: &[u8]) -> bool {
        use core::slice;
        assert!(buf.len() >= ide::BLOCK_SIZE);
        let buf = unsafe { slice::from_raw_parts(buf.as_ptr() as *mut u32, ide::BLOCK_SIZE / 4) };
        self.write(block_id as u64, 1, buf).is_ok()
    }
}