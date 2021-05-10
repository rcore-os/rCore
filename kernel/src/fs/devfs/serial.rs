use crate::drivers::serial::SERIAL_ACTIVITY;
use crate::drivers::SerialDriver;
use crate::drivers::SERIAL_DRIVERS;
use crate::syscall::spin_and_wait;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::any::Any;
use rcore_fs::vfs::*;
pub struct Serial {
    id: usize,
    driver: Arc<dyn SerialDriver>,
}

impl Serial {
    pub fn new(id: usize, driver: Arc<dyn SerialDriver>) -> Self {
        Serial { id, driver }
    }
    pub fn wrap_all_serial_devices() -> Vec<Self> {
        let drivers = SERIAL_DRIVERS.read();
        drivers
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, x)| Serial::new(i, x))
            .collect()
    }
}

impl INode for Serial {
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> Result<usize> {
        let mut n = 0;
        for r in buf.iter_mut() {
            if let Some(x) = self.driver.try_read() {
                *r = x;
                n += 1;
            } else {
                break;
            }
        }
        Ok(n)
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        self.driver.write(buf);
        Ok(buf.len())
    }

    fn poll(&self) -> Result<PollStatus> {
        Ok(PollStatus {
            read: true,
            write: true,
            error: false,
        })
    }

    fn metadata(&self) -> Result<Metadata> {
        Ok(Metadata {
            dev: 1,
            inode: 1,
            size: 0,
            blk_size: 0,
            blocks: 0,
            atime: Timespec { sec: 0, nsec: 0 },
            mtime: Timespec { sec: 0, nsec: 0 },
            ctime: Timespec { sec: 0, nsec: 0 },
            type_: FileType::CharDevice,
            mode: 0o666,
            nlinks: 1,
            uid: 0,
            gid: 0,
            rdev: make_rdev(4, self.id),
        })
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }
}
