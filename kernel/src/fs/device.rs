//! Implement Device

use rcore_fs::dev::*;
use spin::RwLock;

#[cfg(target_arch = "x86_64")]
use crate::arch::driver::ide;

#[cfg(target_arch = "aarch64")]
use crate::arch::board::emmc;
use crate::sync::SpinNoIrqLock as Mutex;

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

#[cfg(target_arch = "x86_64")]
impl BlockDevice for ide::IDE {
    const BLOCK_SIZE_LOG2: u8 = 9;
    fn read_at(&self, block_id: usize, buf: &mut [u8]) -> Result<()> {
        use core::slice;
        assert!(buf.len() >= ide::BLOCK_SIZE);
        let buf =
            unsafe { slice::from_raw_parts_mut(buf.as_ptr() as *mut u32, ide::BLOCK_SIZE / 4) };
        self.read(block_id as u64, 1, buf).map_err(|_| DevError)?;
        Ok(())
    }
    fn write_at(&self, block_id: usize, buf: &[u8]) -> Result<()> {
        use core::slice;
        assert!(buf.len() >= ide::BLOCK_SIZE);
        let buf = unsafe { slice::from_raw_parts(buf.as_ptr() as *mut u32, ide::BLOCK_SIZE / 4) };
        self.write(block_id as u64, 1, buf).map_err(|_| DevError)?;
        Ok(())
    }
    fn sync(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(target_arch = "aarch64")]
pub struct EmmcDriver(Mutex<emmc::EmmcCtl>);

#[cfg(target_arch = "aarch64")]
impl BlockDevice for EmmcDriver {
    const BLOCK_SIZE_LOG2: u8 = 9;
    fn read_at(&self, block_id: usize, buf: &mut [u8]) -> Result<()> {
        use core::slice;
        assert!(buf.len() >= emmc::BLOCK_SIZE);
        let buf =
            unsafe { slice::from_raw_parts_mut(buf.as_ptr() as *mut u32, emmc::BLOCK_SIZE / 4) };
        let mut ctrl = self.0.lock();
        ctrl.read(block_id as u32, 1, buf).map_err(|_| DevError)?;
        Ok(())
    }
    fn write_at(&self, block_id: usize, buf: &[u8]) -> Result<()> {
        use core::slice;
        assert!(buf.len() >= emmc::BLOCK_SIZE);
        let buf = unsafe { slice::from_raw_parts(buf.as_ptr() as *mut u32, emmc::BLOCK_SIZE / 4) };
        let mut ctrl = self.0.lock();
        ctrl.write(block_id as u32, 1, buf).map_err(|_| DevError)?;
        Ok(())
    }
    fn sync(&self) -> Result<()> {
        Ok(())
    }
}
