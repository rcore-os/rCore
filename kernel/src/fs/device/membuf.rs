use rcore_fs::dev::*;
use spin::RwLock;

pub struct MemBuf(RwLock<&'static mut [u8]>);

impl MemBuf {
    pub unsafe fn new(begin: unsafe extern "C" fn(), end: unsafe extern "C" fn()) -> Self {
        use core::slice;
        MemBuf(RwLock::new(slice::from_raw_parts_mut(
            begin as *mut u8,
            end as usize - begin as usize,
        )))
    }
}

impl Device for MemBuf {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        let slice = self.0.read();
        let len = buf.len().min(slice.len() - offset);
        buf[..len].copy_from_slice(&slice[offset..offset + len]);
        Ok(len)
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        let mut slice = self.0.write();
        let len = buf.len().min(slice.len() - offset);
        slice[offset..offset + len].copy_from_slice(&buf[..len]);
        Ok(len)
    }

    fn sync(&self) -> Result<()> {
        Ok(())
    }
}
