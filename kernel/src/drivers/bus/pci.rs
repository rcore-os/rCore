use crate::drivers::net::*;
use crate::drivers::{Driver, DRIVERS, NET_DRIVERS};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use spin::Mutex;
use core::cmp::Ordering;
use x86_64::instructions::port::Port;

const PCI_VENDOR: u32 = 0x00;
const PCI_DEVICE: u32 = 0x02;
const PCI_COMMAND: u32 = 0x04;
const PCI_STATUS: u32 = 0x06;
const PCI_SUBCLASS: u32 = 0x0a;
const PCI_CLASS: u32 = 0x0b;
const PCI_HEADER: u32 = 0x0e;
const PCI_BAR0: u32 = 0x10; // first
const PCI_BAR5: u32 = 0x24; // last
const PCI_CAP_PTR: u32 = 0x34;
const PCI_INTERRUPT_LINE: u32 = 0x3c;
const PCI_INTERRUPT_PIN: u32 = 0x3d;

const PCI_MSI_CTRL_CAP: u32 = 0x00;
const PCI_MSI_ADDR: u32 = 0x04;
const PCI_MSI_UPPER_ADDR: u32 = 0x08;
const PCI_MSI_DATA: u32 = 0x0C;

const PCI_CAP_ID_MSI: u32 = 0x05;

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

#[derive(Copy, Clone)]
pub struct PciTag(u32);


impl Ord for PciTag {
    fn cmp(&self, other: &PciTag) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for PciTag {
    fn partial_cmp(&self, other: &PciTag) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for PciTag {
}

impl PartialEq for PciTag {
    fn eq(&self, other: &PciTag) -> bool {
        self.0 == other.0
    }
}

impl PciTag {
    pub fn new(bus: u32, dev: u32, func: u32) -> PciTag {
        assert!(bus < 256);
        assert!(dev < 32);
        assert!(func < 8);
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
        let m = if width < 4 {
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
    // return (addr, len)
    pub unsafe fn get_bar_mem(&self, bar_number: u32) -> Option<(usize, usize)> {
        assert!(bar_number <= 4);
        let bar = PCI_BAR0 + 4 * bar_number;
        let mut base_lo = self.read(bar, 4);
        self.write(bar, 0xffffffff);
        let mut max_base_lo = self.read(bar, 4);
        self.write(bar, base_lo);

        let mut base = 0usize;
        let mut max_base = 0usize;
        let mut address_mark = 0usize;

        // memory instead of io
        assert!(base_lo & PCI_BASE_ADDRESS_SPACE == PCI_BASE_ADDRESS_SPACE_MEMORY);
        match base_lo & PCI_BASE_ADDRESS_MEM_TYPE_MASK {
            PCI_BASE_ADDRESS_MEM_TYPE_32 => {
                base = (base_lo & PCI_BASE_ADDRESS_MEM_MASK) as usize;
                max_base = (max_base_lo & PCI_BASE_ADDRESS_MEM_MASK) as usize;
                address_mark = PCI_BASE_ADDRESS_MEM_MASK as usize;
            }
            PCI_BASE_ADDRESS_MEM_TYPE_64 => {
                base = (base_lo & PCI_BASE_ADDRESS_MEM_MASK) as usize;
                max_base = (max_base_lo & PCI_BASE_ADDRESS_MEM_MASK) as usize;

                let base_hi = self.read(bar + 4, 4);
                self.write(bar + 4, 0xffffffff);
                let max_base_hi = self.read(bar + 4, 4);
                self.write(bar + 4, base_hi);
                base |= (base_hi as usize) << 32;
                max_base |= (max_base_hi as usize) << 32;
                address_mark = !0;
            }
            _ => unimplemented!("pci bar mem type"),
        }

        if max_base == 0 {
            return None;
        }

        // linux/drivers/pci/probe.c pci_size
        let mut size = max_base & address_mark;
        if size == 0 {
            return None;
        }
        size = (size & !(size - 1)) - 1;

        debug!(
            "device memory address from {:#X} to {:#X}",
            base,
            base + size
        );
        return Some((base as usize, size as usize));
    }

    // returns a tuple of (vid, did, next)
    pub fn probe(&self) -> Option<(u32, u32, bool)> {
        unsafe {
            // To lookup vendor and device, please see https://pci-ids.ucw.cz/read/PC/
            let v = self.read(PCI_VENDOR, 2);
            if v == 0xffff {
                return None;
            }
            let d = self.read(PCI_DEVICE, 2);
            let mf = self.read(PCI_HEADER, 1);

            // To lookup class and subclass, please see https://pci-ids.ucw.cz/read/PD/
            let cl = self.read(PCI_CLASS, 1);
            let scl = self.read(PCI_SUBCLASS, 1);
            let line = self.read(PCI_INTERRUPT_LINE, 1);
            let pin = self.read(PCI_INTERRUPT_PIN, 1);
            info!(
                "{:02x}:{:02x}.{}: {:#x} {:#x} ({} {}) irq {}:{}",
                self.bus(),
                self.dev(),
                self.func(),
                v,
                d,
                cl,
                scl,
                line,
                pin
            );

            return Some((v, d, mf & 0x80 != 0));
        }
    }

    /// Enable the pci tag and its interrupt
    /// Return assigned MSI interrupt number when applicable
    pub unsafe fn enable(&self) -> Option<u32> {
        // 23 and lower are used
        static mut MSI_IRQ: u32 = 23;

        let orig = self.read(PCI_COMMAND, 2);
        // IO Space | MEM Space | Bus Mastering | Special Cycles | PCI Interrupt Disable
        self.write(PCI_COMMAND, orig | 0x40f);

        // find MSI cap
        let mut msi_found = false;
        let mut cap_ptr = self.read(PCI_CAP_PTR, 1);
        let mut assigned_irq = None;
        while cap_ptr > 0 {
            let cap_id = self.read(cap_ptr, 1);
            if cap_id == PCI_CAP_ID_MSI {
                // The manual Volume 3 Chapter 10.11 Message Signalled Interrupts
                // 0 is (usually) the apic id of the bsp.
                self.write(cap_ptr + PCI_MSI_ADDR, 0xfee00000 | (0 << 12));
                MSI_IRQ += 1;
                let irq = MSI_IRQ;
                assigned_irq = Some(irq);
                // we offset all our irq numbers by 32
                self.write(cap_ptr + PCI_MSI_DATA, irq + 32);

                // enable MSI interrupt, assuming 64bit for now
                let orig_ctrl = self.read(cap_ptr + PCI_MSI_CTRL_CAP, 4);
                self.write(cap_ptr + PCI_MSI_CTRL_CAP, orig_ctrl | 0x10000);
                debug!(
                    "MSI control {:#b}, enabling MSI interrupts",
                    orig_ctrl >> 16
                );
                msi_found = true;
                break;
            }
            debug!(
                "PCI device has cap id {} at {:#X}",
                self.read(cap_ptr, 1),
                cap_ptr
            );
            cap_ptr = self.read(cap_ptr + 1, 1);
        }

        if !msi_found {
            // Use PCI legacy interrupt instead
            // IO Space | MEM Space | Bus Mastering | Special Cycles
            self.write(PCI_COMMAND, orig | 0xf);
            let line = self.read(PCI_INTERRUPT_LINE, 1);
            let pin = self.read(PCI_INTERRUPT_PIN, 1);
            debug!(
                "MSI not found, using PCI interrupt line {} pin {}",
                line, pin
            );
        }

        assigned_irq
    }
}

pub fn init_driver(name: String, vid: u32, did: u32, tag: PciTag) {
    if vid == 0x8086 {
        if did == 0x100e || did == 0x100f || did == 0x10d3 {
            // 0x100e
            // 82540EM Gigabit Ethernet Controller
            // 0x100f
            // 82545EM Gigabit Ethernet Controller (Copper)
            // 0x10d3
            // 82574L Gigabit Network Connection
            if let Some((addr, len)) = unsafe { tag.get_bar_mem(0) } {
                unsafe {
                    tag.enable();
                }
                e1000::e1000_init(addr, len);
            }
        } else if did == 0x10fb {
            // 82599ES 10-Gigabit SFI/SFP+ Network Connection
            if let Some((addr, len)) = unsafe { tag.get_bar_mem(0) } {
                let irq = unsafe { tag.enable() };
                PCI_DRIVERS.lock()
                    .insert(tag, ixgbe::ixgbe_init(name, irq, addr, len));
            }
        }
    }
}

pub fn detach_driver(bus: u32, dev: u32, func: u32) -> bool {
    match PCI_DRIVERS.lock().remove(&PciTag::new(bus, dev, func)) {
        Some(driver) => {
            DRIVERS.write().retain(|dri| !Arc::ptr_eq(&driver, dri));
            NET_DRIVERS.write().retain(|dri| !Arc::ptr_eq(&driver, dri));
            true
        }
        None => {
            false
        }
    }
}

pub fn init() {
    for bus in 0..256 {
        for dev in 0..32 {
            let tag = PciTag::new(bus, dev, 0);
            if let Some((vid, did, next)) = tag.probe() {
                let name = format!("enp{}s{}f0", bus, dev);
                init_driver(name, vid, did, tag);
                if next {
                    for func in 1..8 {
                        let tag = PciTag::new(bus, dev, func);
                        if let Some((vid, did, _)) = tag.probe() {
                            let name = format!("enp{}s{}f{}", bus, dev, func);
                            init_driver(name, vid, did, tag);
                        }
                    }
                }
            }
        }
    }
}

pub fn find_device(vendor: u32, product: u32) -> Option<PciTag> {
    for bus in 0..256 {
        for dev in 0..32 {
            let tag = PciTag::new(bus, dev, 0);
            if let Some((vid, did, next)) = tag.probe() {
                if vid == vendor && did == product {
                    return Some(tag);
                }
                if next {
                    for func in 1..8 {
                        let tag = PciTag::new(bus, dev, func);
                        if let Some((vid, did, _)) = tag.probe() {
                            if vid == vendor && did == product {
                                return Some(tag);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

lazy_static! {
    pub static ref PCI_DRIVERS: Arc<Mutex<BTreeMap<PciTag, Arc<Driver>>>> = Arc::new(Mutex::new(BTreeMap::new()));
}
