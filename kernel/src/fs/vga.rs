use rcore_fs::vfs::*;

use crate::arch::board::fb::FRAME_BUFFER;
use crate::memory::phys_to_virt;
use alloc::{string::String, sync::Arc, vec::Vec};
use core::any::Any;

#[derive(Default)]
pub struct Vga;

macro_rules! impl_inode {
    () => {
        fn set_metadata(&self, _metadata: &Metadata) -> Result<()> { Ok(()) }
        fn sync_all(&self) -> Result<()> { Ok(()) }
        fn sync_data(&self) -> Result<()> { Ok(()) }
        fn resize(&self, _len: usize) -> Result<()> { Err(FsError::NotSupported) }
        fn create(&self, _name: &str, _type_: FileType, _mode: u32) -> Result<Arc<INode>> { Err(FsError::NotDir) }
        fn unlink(&self, _name: &str) -> Result<()> { Err(FsError::NotDir) }
        fn link(&self, _name: &str, _other: &Arc<INode>) -> Result<()> { Err(FsError::NotDir) }
        fn move_(&self, _old_name: &str, _target: &Arc<INode>, _new_name: &str) -> Result<()> { Err(FsError::NotDir) }
        fn find(&self, _name: &str) -> Result<Arc<INode>> { Err(FsError::NotDir) }
        fn get_entry(&self, _id: usize) -> Result<String> { Err(FsError::NotDir) }
        fn io_control(&self, cmd: u32, data: usize) -> Result<()> { Err(FsError::NotSupported) }
        fn fs(&self) -> Arc<FileSystem> { unimplemented!() }
        fn as_any_ref(&self) -> &Any { self }
    };
}

impl INode for Vga {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        Err(FsError::NotSupported)
    }
    fn write_at(&self, _offset: usize, _buf: &[u8]) -> Result<usize> {
        info!("the _offset is {} {}", _offset, _buf[0]);
        use core::slice;
        let frame_buffer_data = unsafe {
            slice::from_raw_parts_mut(
                phys_to_virt(0xfd00_0000) as *mut u8,
                (1024 * 768 * 3) as usize,
            )
        };
        frame_buffer_data.copy_from_slice(&_buf);
        return Ok(1024 * 768 * 3);
    }
    fn poll(&self) -> Result<PollStatus> {
        Ok(PollStatus {
            // TOKNOW and TODO
            read: true,
            write: false,
            error: false,
        })
    }
    fn metadata(&self) -> Result<Metadata> {
        Ok(Metadata {
            dev: 0,
            inode: 0,
            size: 0x24000,
            blk_size: 0,
            blocks: 0,
            atime: Timespec { sec: 0, nsec: 0 },
            mtime: Timespec { sec: 0, nsec: 0 },
            ctime: Timespec { sec: 0, nsec: 0 },
            type_: FileType::SymLink,
            mode: 0,
            nlinks: 0,
            uid: 0,
            gid: 0,
        })
    }
    impl_inode!();
}
