use alloc::{sync::Arc, vec::Vec};

use rcore_fs::vfs::*;
use rcore_fs::dev::block_cache::BlockCache;
use rcore_fs_sfs::SimpleFileSystem;

use crate::drivers::BlockDriver;

pub use self::file::*;
pub use self::file_like::*;
pub use self::pipe::Pipe;
pub use self::pseudo::*;
pub use self::stdio::{STDIN, STDOUT};

mod device;
mod file;
mod file_like;
mod pipe;
mod pseudo;
mod stdio;

/// Hard link user programs
#[cfg(feature = "link_user")]
global_asm!(concat!(
    r#"
	.section .data.img
	.global _user_img_start
	.global _user_img_end
_user_img_start:
    .incbin ""#,
    env!("SFSIMG"),
    r#""
_user_img_end:
"#
));

lazy_static! {
    /// The root of file system
    pub static ref ROOT_INODE: Arc<INode> = {
        #[cfg(not(feature = "link_user"))]
        let device = {
            #[cfg(any(target_arch = "riscv32", target_arch = "riscv64", target_arch = "x86_64"))]
            {
                let driver = BlockDriver(
                    crate::drivers::BLK_DRIVERS
                        .read().iter()
                        .next().expect("Block device not found")
                        .clone()
                );
                // enable block cache
                // Arc::new(BlockCache::new(driver, 0x100))
                Arc::new(driver)
            }
            #[cfg(target_arch = "aarch64")]
            {
                unimplemented!()
            }
        };
        #[cfg(feature = "link_user")]
        let device = {
            extern {
                fn _user_img_start();
                fn _user_img_end();
            }
            info!("SFS linked to kernel, from {:08x} to {:08x}", _user_img_start as usize, _user_img_end as usize);
            Arc::new(unsafe { device::MemBuf::new(_user_img_start, _user_img_end) })
        };

        let sfs = SimpleFileSystem::open(device).expect("failed to open SFS");
        sfs.root_inode()
    };
}

pub const FOLLOW_MAX_DEPTH: usize = 1;

pub trait INodeExt {
    fn read_as_vec(&self) -> Result<Vec<u8>>;
}

impl INodeExt for INode {
    fn read_as_vec(&self) -> Result<Vec<u8>> {
        let size = self.metadata()?.size;
        let mut buf = Vec::with_capacity(size);
        unsafe {
            buf.set_len(size);
        }
        self.read_at(0, buf.as_mut_slice())?;
        Ok(buf)
    }
}
