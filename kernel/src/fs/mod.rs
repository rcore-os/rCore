use alloc::{boxed::Box, collections::VecDeque, string::String, sync::Arc, vec::Vec};
use core::any::Any;
use core::ops::Deref;

use rcore_fs::vfs::*;
use rcore_fs_sfs::SimpleFileSystem;

#[cfg(target_arch = "x86_64")]
use crate::arch::driver::ide;
use crate::drivers::{self, AsAny};
use crate::drivers::block::virtio_blk::VirtIOBlkDriver;

pub use self::file::*;
pub use self::stdio::{STDIN, STDOUT};

mod file;
mod stdio;
mod device;

lazy_static! {
    /// The root of file system
    pub static ref ROOT_INODE: Arc<INode> = {
        #[cfg(not(feature = "link_user"))]
        let device = {
            #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
            {
                Box::new(drivers::DRIVERS.lock().iter()
                    .map(|device| device.deref().as_any().downcast_ref::<VirtIOBlkDriver>())
                    .find(|maybe_blk| maybe_blk.is_some())
                    .expect("VirtIOBlk not found")
                    .unwrap().clone())
            }
            #[cfg(target_arch = "x86_64")]
            {
                Box::new(ide::IDE::new(1))
            }
        };
        #[cfg(feature = "link_user")]
        let device = {
            extern {
                fn _user_img_start();
                fn _user_img_end();
            }
            Box::new(unsafe { device::MemBuf::new(_user_img_start, _user_img_end) })
        };

        let sfs = SimpleFileSystem::open(device).expect("failed to open SFS");
        sfs.root_inode()
    };
}

pub trait INodeExt {
    fn read_as_vec(&self) -> Result<Vec<u8>>;
}

impl INodeExt for INode {
    fn read_as_vec(&self) -> Result<Vec<u8>> {
        let size = self.metadata()?.size;
        let mut buf = Vec::with_capacity(size);
        unsafe { buf.set_len(size); }
        self.read_at(0, buf.as_mut_slice())?;
        Ok(buf)
    }
}
