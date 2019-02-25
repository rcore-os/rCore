use x86_64::instructions::port::Port;
use crate::logging::*;
use core::slice;

const VENDOR: u32 = 0x00;
const DEVICE: u32 = 0x02;
const STATUS: u32 = 0x06;
const SUBCLASS: u32 = 0x0a;
const CLASS: u32 = 0x0b;
const HEADER: u32 = 0x0e;
const BAR0: u32 = 0x10;

const PCI_ADDR_PORT: u16 = 0xcf8;
const PCI_DATA_PORT: u16 = 0xcfc;

const PCI_BASE_ADDRESS_SPACE: u32 = 0x01;
const PCI_BASE_ADDRESS_SPACE_IO: u32 = 0x01;
const PCI_BASE_ADDRESS_SPACE_MEMORY: u32 = 0x00;

const PCI_BASE_ADDRESS_MEM_TYPE_MASK: u32 = 0x06;
const PCI_BASE_ADDRESS_MEM_TYPE_32: u32 = 0x00;
const PCI_BASE_ADDRESS_MEM_TYPE_1M: u32 = 0x02;
const PCI_BASE_ADDRESS_MEM_TYPE_64: u32 = 0x04;
const PCI_BASE_ADDRESS_MEM_PREFETCH: u32 = 0x08;
const PCI_BASE_ADDRESS_MEM_MASK: u32 = 0xfffffff0;

struct PciTag(u32);

impl PciTag {
    pub fn new(bus: u32, dev: u32, func: u32) -> PciTag {
        PciTag(bus << 16 | dev << 11 | func << 8)
    }

    pub fn bus(&self) -> u32 {
        (self.0 >> 16) & 0xFF
    }

    pub fn dev(&self) -> u32 {
        (self.0 >> 11) & 0x1F
    }

    pub fn func(&self) -> u32 {
        (self.0 >> 8) & 0x7
    }

    // biscuit/src/pci/pci.go Pci_read
    pub unsafe fn read(&self, reg: u32, width: u32) -> u32 {
        // spans in one reg
        assert_eq!(reg / 4, (reg + width - 1) / 4);

        let enable = 1 << 31;
        let rsh = reg % 4;
        let r = reg - rsh;
        let t = enable | self.0 | r;

        let mut pci_addr: Port<u32> = Port::new(PCI_ADDR_PORT);
        let mut pci_data: Port<u32> = Port::new(PCI_DATA_PORT);

        pci_addr.write(t);
        let d = pci_data.read();
        pci_addr.write(0);

        let ret = d >> (rsh * 8);
        let m = if (width < 4) {
            (1 << (8 * width)) - 1
        } else {
            0xffffffff
        };

        return ret & m;
    }

    pub unsafe fn write(&self, reg: u32, val: u32) {
        assert_eq!(reg & 3, 0);

        let enable = 1 << 31;
        let t = enable | self.0 | reg;

        let mut pci_addr: Port<u32> = Port::new(PCI_ADDR_PORT);
        let mut pci_data: Port<u32> = Port::new(PCI_DATA_PORT);

        pci_addr.write(t);
        pci_data.write(val);
        pci_addr.write(0);
    }

    // biscuit/src/pci/pci.go Pci_bar_mem
    // linux/drivers/pci/probe.c pci_read_bases
    pub unsafe fn getBarMem(&self, bar_number: u32) -> Option<&'static mut [u8]> {
        assert!(bar_number <= 4);
        let bar = BAR0 + 4 * bar_number;
        let mut base = self.read(bar, 4);
        self.write(bar, 0xffffffff);
        let mut max_base = self.read(bar, 4);
        self.write(bar, base);

        // memory instead of io
        assert!(base & PCI_BASE_ADDRESS_SPACE == PCI_BASE_ADDRESS_SPACE_MEMORY);
        // only support 32bit addr for now
        assert!(base & PCI_BASE_ADDRESS_MEM_TYPE_MASK == PCI_BASE_ADDRESS_MEM_TYPE_32);

        base = base & PCI_BASE_ADDRESS_MEM_MASK;
        max_base = max_base & PCI_BASE_ADDRESS_MEM_MASK;

        if (max_base == 0) {
            return None;
        }

        // linux/drivers/pci/probe.c pci_size
        let mut size = PCI_BASE_ADDRESS_MEM_MASK & max_base;
        if (size == 0) {
            return None;
        }
        size = (size & !(size - 1)) - 1;

        debug!("device memory address from {:#X} to {:#X}", base, base + size);
        return Some(slice::from_raw_parts_mut(base as *mut u8, size as usize));
    }

    pub fn describe(&self) -> bool {
        unsafe {
            let v = self.read(VENDOR, 2);
            if v == 0xffff {
                return false;
            }
            let d = self.read(DEVICE, 2);
            let mf = self.read(HEADER, 1);
            let cl = self.read(CLASS, 1);
            let scl = self.read(SUBCLASS, 1);
            info!("{}: {}: {}: {:#X} {:#X} ({} {})", self.bus(), self.dev(), self.func(), v, d, cl, scl);
            self.getBarMem(0);
            return mf & 0x80 != 0;
        }
    }
}

pub fn init() {
    for bus in 0..256 {
        for dev in 0..32 {
            let tag = PciTag::new(bus, dev, 0);
            if tag.describe() {
                for func in 1..8 {
                    let tag = PciTag::new(bus, dev, func);
                    tag.describe();
                }
            }
        }
    }
    info!("Init pci");
}