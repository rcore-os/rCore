use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::{Debug, Error, Formatter};
use rcore_fs::dev::{self, Device};

pub struct Partition {
    id: usize,
    offset: usize,
    size: usize,
    device: Arc<dyn Device>,
}

impl Device for Partition {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> dev::Result<usize> {
        self.device.read_at(self.offset + offset, buf)
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> dev::Result<usize> {
        self.device.write_at(self.offset + offset, buf)
    }

    fn sync(&self) -> dev::Result<()> {
        self.device.sync()
    }
}

impl Partition {
    pub fn new(id: usize, offset: usize, size: usize, device: Arc<dyn Device>) -> Self {
        Partition {
            id,
            offset,
            size,
            device,
        }
    }
}

impl Debug for Partition {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_struct("Partition")
            .field("id", &self.id)
            .field("offset", &self.offset)
            .field("size", &self.size)
            .finish()
    }
}

pub struct Disk {
    parts: Vec<Arc<Partition>>,
    device: Arc<dyn Device>,
}

impl Device for Disk {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> dev::Result<usize> {
        self.device.read_at(offset, buf)
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> dev::Result<usize> {
        self.device.write_at(offset, buf)
    }

    fn sync(&self) -> dev::Result<()> {
        self.device.sync()
    }
}

impl Disk {
    pub fn new(device: Arc<dyn Device>, size: usize) -> Self {
        let part = Arc::new(Partition::new(0, 0, size, device.clone()));
        Self {
            parts: vec![part],
            device,
        }
    }

    pub fn partition_iter(&self) -> impl Iterator<Item = &Arc<Partition>> {
        self.parts.iter()
    }
}

impl Debug for Disk {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_struct("Disk").field("parts", &self.parts).finish()
    }
}
