use super::Driver;

pub mod ahci;
pub mod ide;
pub mod virtio_blk;

pub trait BlockDriver: Driver {
    fn read_block(&self, _block_id: usize, _buf: &mut [u8]) -> bool {
        unimplemented!("not a block driver")
    }

    fn write_block(&self, _block_id: usize, _buf: &[u8]) -> bool {
        unimplemented!("not a block driver")
    }
}
