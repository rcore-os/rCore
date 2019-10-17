use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::{Debug, Error, Formatter};
use core::mem::{size_of_val, uninitialized};
use core::slice;
use rcore_fs::dev::{self, DevError, Device};

const SECTOR_SIZE: usize = 512;
const PARTITION_TABLE_OFFSET: usize = 446;
const MBR_SIGNATURE: u16 = 0xAA55;

#[repr(packed)]
#[derive(Debug)]
struct PartitionRecord {
    /// 0x80 - active
    boot_ind: u8,
    /// starting head
    head: u8,
    /// starting sector
    sector: u8,
    /// starting cylinder
    cyl: u8,
    /// What partition type
    sys_int: u8,
    /// end head
    end_head: u8,
    /// end sector
    end_sector: u8,
    /// end cylinder
    end_cyl: u8,
    /// starting sector counting from
    start_sect: u32,
    /// number of sectors in partition
    nr_sects: u32,
}

#[repr(packed)]
struct MBRHeader {
    /// bootstrap code area
    _unsued: [u8; PARTITION_TABLE_OFFSET],
    /// partition records
    part_records: [PartitionRecord; 4],
    /// must equals `0xAA55`
    signature: u16,
}

impl Debug for MBRHeader {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_struct("MBRHeader")
            .field("part_records", &self.part_records)
            .field("signature", &self.signature)
            .finish()
    }
}

pub struct Partition {
    /// index
    id: usize,
    /// starting position (in bytes)
    offset: usize,
    /// total size (in bytes)
    size: usize,
    /// lower level disk device
    device: Arc<dyn Device>,
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
    /// partitions at this disk
    parts: Vec<Arc<Partition>>,
    /// lower level disk device
    device: Arc<dyn Device>,
}

impl Disk {
    fn mbr_paritions(device: Arc<dyn Device>) -> dev::Result<Vec<Arc<Partition>>> {
        let mut header: MBRHeader = unsafe { uninitialized() };
        device.read_at(0, unsafe {
            slice::from_raw_parts_mut(&mut header as *mut _ as *mut u8, size_of_val(&header))
        })?;

        if header.signature == MBR_SIGNATURE {
            let mut parts = Vec::new();
            for (i, p) in header.part_records.iter().enumerate() {
                if p.sys_int != 0 {
                    let part = Partition::new(
                        i,
                        p.start_sect as usize * SECTOR_SIZE,
                        p.nr_sects as usize * SECTOR_SIZE,
                        device.clone(),
                    );
                    parts.push(Arc::new(part));
                }
            }
            Ok(parts)
        } else {
            Err(DevError)
        }
    }

    pub fn new(device: Arc<dyn Device>, size: usize) -> Self {
        let parts = if let Ok(parts) = Self::mbr_paritions(device.clone()) {
            parts
        } else {
            vec![Arc::new(Partition::new(0, 0, size, device.clone()))]
        };
        Disk { parts, device }
    }

    pub fn partition_iter(&self) -> impl Iterator<Item = &Arc<Partition>> {
        self.parts.iter()
    }
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

impl Debug for Disk {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_struct("Disk").field("parts", &self.parts).finish()
    }
}
