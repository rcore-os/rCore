//! ATA IO code, handling device multiplexing and IO operations
//!
//! Borrow from Rucore project. Thanks GWord!
//! Port from ucore C code.
//! Currently used in MIPS

use crate::drivers::{block::BlockDriver, DeviceType, Driver};
use crate::drivers::{BLK_DRIVERS, DRIVERS};
use crate::sync::SpinNoIrqLock as Mutex;
use alloc::string::String;
use alloc::sync::Arc;
use core::slice;

pub const BLOCK_SIZE: usize = 512;

pub struct IDE {
    num: u8,
    /// I/O Base
    base: u16,
    /// Control Base
    ctrl: u16,
}

pub struct IDEDriver(Mutex<IDE>);

impl Driver for IDEDriver {
    fn try_handle_interrupt(&self, _irq: Option<usize>) -> bool {
        false
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Block
    }

    fn get_id(&self) -> String {
        format!("ide")
    }

    fn as_block(&self) -> Option<&dyn BlockDriver> {
        Some(self)
    }
}

impl BlockDriver for IDEDriver {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) -> bool {
        let mut driver = self.0.lock();
        let buf = unsafe { slice::from_raw_parts_mut(buf.as_ptr() as *mut u32, BLOCK_SIZE / 4) };
        driver.read(block_id as u64, 1, buf).is_ok()
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) -> bool {
        if buf.len() < BLOCK_SIZE {
            return false;
        }
        let mut driver = self.0.lock();
        let buf = unsafe { slice::from_raw_parts(buf.as_ptr() as *mut u32, BLOCK_SIZE / 4) };
        driver.write(block_id as u64, 1, buf).is_ok()
    }
}

impl IDE {
    pub fn new(num: u8) -> Self {
        let ide = match num {
            0 => IDE {
                num: 0,
                base: 0x1f0,
                ctrl: 0x3f4,
            },
            1 => IDE {
                num: 1,
                base: 0x1f0,
                ctrl: 0x3f4,
            },
            2 => IDE {
                num: 2,
                base: 0x170,
                ctrl: 0x374,
            },
            3 => IDE {
                num: 3,
                base: 0x170,
                ctrl: 0x374,
            },
            _ => panic!("ide number should be 0,1,2,3"),
        };
        ide.init();
        ide
    }

    /// Read ATA DMA. Block size = 512 bytes.
    pub fn read(&self, sector: u64, count: usize, data: &mut [u32]) -> Result<(), ()> {
        assert_eq!(data.len(), count * SECTOR_SIZE);
        self.wait();
        unsafe {
            self.select(sector, count as u8);
            port::outb(self.base + ISA_COMMAND, IDE_CMD_READ);
            for i in 0..count {
                let ptr = &mut data[(i as usize) * SECTOR_SIZE];
                if self.wait_error() {
                    return Err(());
                }
                insl(self.base, ptr, SECTOR_SIZE);
            }
        }
        Ok(())
    }
    /// Write ATA DMA. Block size = 512 bytes.
    pub fn write(&self, sector: u64, count: usize, data: &[u32]) -> Result<(), ()> {
        assert_eq!(data.len(), count * SECTOR_SIZE);
        self.wait();
        unsafe {
            self.select(sector, count as u8);
            port::outb(self.base + ISA_COMMAND, IDE_CMD_WRITE);
            for i in 0..count {
                let ptr = &data[(i as usize) * SECTOR_SIZE];
                if self.wait_error() {
                    return Err(());
                }
                outsl(self.base, ptr, SECTOR_SIZE);
            }
        }
        Ok(())
    }

    fn wait(&self) {
        while unsafe { port::inb(self.base + ISA_STATUS) } & IDE_BUSY != 0 {}
    }

    fn wait_error(&self) -> bool {
        self.wait();
        let status = unsafe { port::inb(self.base + ISA_STATUS) };
        status & (IDE_DF | IDE_ERR) != 0
    }

    fn init(&self) {
        self.wait();
        unsafe {
            // step1: select drive
            port::outb(self.base + ISA_SDH, (0xE0 | ((self.num & 1) << 4)) as u8);
            self.wait();

            // step2: send ATA identify command
            port::outb(self.base + ISA_COMMAND, IDE_CMD_IDENTIFY);
            self.wait();

            // step3: polling
            if port::inb(self.base + ISA_STATUS) == 0 || self.wait_error() {
                return;
            }

            // ???
            let mut data = [0; SECTOR_SIZE];
            insl(self.base + ISA_DATA, data.as_mut_ptr(), SECTOR_SIZE);
        }
    }

    fn select(&self, sector: u64, count: u8) {
        assert_ne!(count, 0);
        self.wait();
        unsafe {
            // generate interrupt
            port::outb(self.ctrl + ISA_CTRL, 0);
            port::outb(self.base + ISA_SECCNT, count);
            port::outb(self.base + ISA_SECTOR, (sector & 0xFF) as u8);
            port::outb(self.base + ISA_CYL_LO, ((sector >> 8) & 0xFF) as u8);
            port::outb(self.base + ISA_CYL_HI, ((sector >> 16) & 0xFF) as u8);
            port::outb(
                self.base + ISA_SDH,
                0xE0 | ((self.num & 1) << 4) | (((sector >> 24) & 0xF) as u8),
            );
        }
    }
}

const SECTOR_SIZE: usize = 128;
const MAX_DMA_SECTORS: usize = 0x1F_F000 / SECTOR_SIZE; // Limited by sector count (and PRDT entries)
                                                        // 512 PDRT entries, assume maximum fragmentation = 512 * 4K max = 2^21 = 2MB per transfer

const ISA_DATA: u16 = 0x00;
const ISA_ERROR: u16 = 0x01;
const ISA_PRECOMP: u16 = 0x01;
const ISA_CTRL: u16 = 0x02;
const ISA_SECCNT: u16 = 0x02;
const ISA_SECTOR: u16 = 0x03;
const ISA_CYL_LO: u16 = 0x04;
const ISA_CYL_HI: u16 = 0x05;
const ISA_SDH: u16 = 0x06;
const ISA_COMMAND: u16 = 0x07;
const ISA_STATUS: u16 = 0x07;

const IDE_BUSY: u8 = 0x80;
const IDE_DRDY: u8 = 0x40;
const IDE_DF: u8 = 0x20;
const IDE_DRQ: u8 = 0x08;
const IDE_ERR: u8 = 0x01;

const IDE_CMD_READ: u8 = 0x20;
const IDE_CMD_WRITE: u8 = 0x30;
const IDE_CMD_IDENTIFY: u8 = 0xEC;

const MAX_NSECS: usize = 128;

#[cfg(target_arch = "x86_64")]
unsafe fn insl(addr: u16, data: *mut u32, len: usize) {
    llvm_asm!("rep insl" :: "{dx}"(addr), "{rdi}"(data), "{cx}"(SECTOR_SIZE) : "rdi" : "volatile");
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn insl(addr: u16, data: *mut u32, len: usize) {
    for i in 0..len {
        *data.add(i) = port::inl(addr);
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn outsl(addr: u16, data: *const u32, len: usize) {
    llvm_asm!("rep insl" :: "{dx}"(addr), "{rdi}"(data), "{cx}"(SECTOR_SIZE) : "rdi" : "volatile");
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn outsl(addr: u16, data: *const u32, len: usize) {
    for i in 0..len {
        port::outl(addr, *data.add(i));
    }
}

// MIPS
mod port {
    const PCI_BASE: usize = 0xb8000000;
    pub unsafe fn inb(port: u16) -> u8 {
        crate::util::read(port as usize + PCI_BASE)
    }

    pub unsafe fn outb(port: u16, value: u8) {
        crate::util::write(port as usize + PCI_BASE, value)
    }

    pub unsafe fn inl(port: u16) -> u32 {
        crate::util::read(port as usize + PCI_BASE)
    }

    pub unsafe fn outl(port: u16, value: u32) {
        crate::util::write(port as usize + PCI_BASE, value)
    }
}

pub fn init() {
    let ide = IDE::new(0);
    let driver = Arc::new(IDEDriver(Mutex::new(ide)));
    DRIVERS.write().push(driver.clone());
    BLK_DRIVERS.write().push(driver.clone());
}
