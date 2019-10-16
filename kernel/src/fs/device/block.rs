use crate::drivers::Driver;
use alloc::sync::Arc;
use rcore_fs::dev::{self, BlockDevice, DevError};

pub struct BlockDriver(pub Arc<dyn Driver>);

impl BlockDevice for BlockDriver {
    const BLOCK_SIZE_LOG2: u8 = 9; // 512

    fn read_at(&self, block_id: usize, buf: &mut [u8]) -> dev::Result<()> {
        match self.0.read_block(block_id, buf) {
            true => Ok(()),
            false => Err(DevError),
        }
    }

    fn write_at(&self, block_id: usize, buf: &[u8]) -> dev::Result<()> {
        match self.0.write_block(block_id, buf) {
            true => Ok(()),
            false => Err(DevError),
        }
    }

    fn sync(&self) -> dev::Result<()> {
        Ok(())
    }
}
