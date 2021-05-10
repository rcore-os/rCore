use alloc::{sync::Arc, vec::Vec};

use rcore_fs::{dev::block_cache::BlockCache, vfs::*};
use rcore_fs_devfs::{
    special::{NullINode, ZeroINode},
    DevFS,
};
use rcore_fs_mountfs::MountFS;
use rcore_fs_ramfs::RamFS;
use rcore_fs_sfs::{INodeImpl, SimpleFileSystem};

use self::devfs::{Fbdev, RandomINode};

pub use self::devfs::{Serial, ShmINode, TTY};
pub use self::file::*;
pub use self::file_like::*;
pub use self::pipe::Pipe;
pub use self::pseudo::*;
use crate::drivers::{BlockDriver, BlockDriverWrapper};

mod devfs;
mod device;
pub mod epoll;
pub mod fcntl;
mod file;
mod file_like;
pub mod ioctl;
mod pipe;
mod pseudo;

// Hard link user programs
#[cfg(feature = "link_user")]
global_asm!(concat!(
    r#"
	.section .data.img
	.global _user_img_start
	.global _user_img_end
_user_img_start:
    .incbin ""#,
    env!("USER_IMG"),
    r#""
_user_img_end:
"#
));

lazy_static! {
    /// The root of file system
    pub static ref ROOT_INODE: Arc<dyn INode> = {
        #[cfg(not(feature = "link_user"))]
        let device = {
            let driver = BlockDriverWrapper(
                crate::drivers::BLK_DRIVERS
                    .read().iter()
                    .next().expect("Block device not found")
                    .clone()
            );
            // enable block cache
            Arc::new(BlockCache::new(driver, 0x100))
            // Arc::new(driver)
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

        // use SFS as rootfs
        let sfs = SimpleFileSystem::open(device).expect("failed to open SFS");
        let rootfs = MountFS::new(sfs);
        let root = rootfs.root_inode();

        // create DevFS
        let devfs = DevFS::new();
        devfs.add("null", Arc::new(NullINode::default())).expect("failed to mknod /dev/null");
        devfs.add("zero", Arc::new(ZeroINode::default())).expect("failed to mknod /dev/zero");
        devfs.add("random", Arc::new(RandomINode::new(false))).expect("failed to mknod /dev/random");
        devfs.add("urandom", Arc::new(RandomINode::new(true))).expect("failed to mknod /dev/urandom");
        devfs.add("tty", TTY.clone()).expect("failed to mknod /dev/tty");
        devfs.add("fb0", Arc::new(Fbdev::default())).expect("failed to mknod /dev/fb0");
        devfs.add("shm", Arc::new(ShmINode::default())).expect("failed to mkdir shm");
        for (i, serial) in Serial::wrap_all_serial_devices().into_iter().enumerate(){
            devfs.add(&format!("ttyS{}", i), Arc::new(serial)).expect("failed to add a serial");
        }


        #[cfg(feature = "hypervisor")]
        devfs.add("rvm", Arc::new(crate::rvm::RvmINode::new())).expect("failed to mknod /dev/rvm");

        // mount DevFS at /dev
        let dev = root.find(true, "dev").unwrap_or_else(|_| {
            root.create("dev", FileType::Dir, 0o666).expect("failed to mkdir /dev")
        });
        let devfs = dev.mount(devfs).expect("failed to mount DevFS");

        // mount RamFS at /dev/shm
        let shm = devfs.root_inode().find(true, "shm").expect("cannot find shm");
        let shmfs = RamFS::new();
        shm.mount(shmfs).expect("failed to mount /dev/shm");

        // mount RamFS at /tmp
        let ramfs = RamFS::new();
        let tmp = root.find(true, "tmp").unwrap_or_else(|_| {
            root.create("tmp", FileType::Dir, 0o666).expect("failed to mkdir /tmp")
        });
        tmp.mount(ramfs).expect("failed to mount RamFS");

        root
    };
}

pub const FOLLOW_MAX_DEPTH: usize = 3;

pub trait INodeExt {
    fn read_as_vec(&self) -> Result<Vec<u8>>;
}

impl INodeExt for dyn INode {
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
