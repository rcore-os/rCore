use x86_64::instructions::port::Port;
use crate::logging::*;

const VENDOR: u32 = 0x00;
const DEVICE: u32 = 0x02;
const STATUS: u32 = 0x06;
const SUBCLASS: u32 = 0x0a;
const CLASS: u32 = 0x0b;
const HEADER: u32 = 0x0e;

const PCI_ADDR_PORT: u16 = 0xcf8;
const PCI_DATA_PORT: u16 = 0xcfc;

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

    // biscuit/src/pci/pci.go
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
        let m = (1 << (8 * width)) - 1;
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