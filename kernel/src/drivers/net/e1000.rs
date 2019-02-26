use alloc::alloc::{GlobalAlloc, Layout};
use alloc::format;
use alloc::prelude::*;
use alloc::sync::Arc;
use core::mem::size_of;
use core::slice;
use core::sync::atomic::{fence, Ordering};

use crate::arch::consts::{KERNEL_OFFSET, MEMORY_OFFSET};
use bitflags::*;
use device_tree::util::SliceRead;
use device_tree::Node;
use log::*;
use rcore_memory::paging::PageTable;
use rcore_memory::PAGE_SIZE;
use smoltcp::phy::{self, DeviceCapabilities};
use smoltcp::time::Instant;
use smoltcp::wire::EthernetAddress;
use smoltcp::Result;
use volatile::{ReadOnly, Volatile};

use crate::arch::cpu;
use crate::memory::active_table;
use crate::sync::SpinNoIrqLock as Mutex;
use crate::HEAP_ALLOCATOR;

use super::super::{DeviceType, Driver, NetDriver, DRIVERS, NET_DRIVERS};

pub struct E1000 {
    header: usize,
    size: usize,
    mac: EthernetAddress,
    send_page: usize,
    send_buffers: Vec<usize>,
    recv_page: usize,
    recv_buffers: Vec<usize>,
    first_trans: bool,
}

const E1000_STATUS: usize = 0x0008 / 4;
const E1000_IMS: usize = 0x00D0 / 4;
const E1000_RCTL: usize = 0x0100 / 4;
const E1000_TCTL: usize = 0x0400 / 4;
const E1000_TIPG: usize = 0x0410 / 4;
const E1000_RDBAL: usize = 0x2800 / 4;
const E1000_RDBAH: usize = 0x2804 / 4;
const E1000_RDLEN: usize = 0x2808 / 4;
const E1000_RDH: usize = 0x2810 / 4;
const E1000_RDT: usize = 0x2818 / 4;
const E1000_TDBAL: usize = 0x3800 / 4;
const E1000_TDBAH: usize = 0x3804 / 4;
const E1000_TDLEN: usize = 0x3808 / 4;
const E1000_TDH: usize = 0x3810 / 4;
const E1000_TDT: usize = 0x3818 / 4;
const E1000_MTA: usize = 0x5200 / 4;
const E1000_RAL: usize = 0x5400 / 4;
const E1000_RAH: usize = 0x5404 / 4;

#[derive(Clone)]
pub struct E1000Driver(Arc<Mutex<E1000>>);

impl Driver for E1000Driver {
    fn try_handle_interrupt(&mut self) -> bool {
        let driver = self.0.lock();

        // ensure header page is mapped
        active_table().map_if_not_exists(driver.header, driver.size);

        return false;
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Net
    }
}

impl E1000 {
    fn transmit_available(&self) -> bool {
        // TODO map it in all cpu
        let mut current_addr = self.header;
        while current_addr < self.header + self.size {
            active_table().map_if_not_exists(current_addr, current_addr);
            current_addr = current_addr + PAGE_SIZE;
        }

        let e1000 =
            unsafe { slice::from_raw_parts_mut(self.header as *mut Volatile<u32>, self.size / 4) };
        let send_queue_size = PAGE_SIZE / size_of::<E1000SendDesc>();
        let mut send_queue = unsafe {
            slice::from_raw_parts_mut(self.send_page as *mut E1000RecvDesc, send_queue_size)
        };
        let mut tdt = e1000[E1000_TDT].read();
        let index = (tdt as usize + 1) % send_queue_size;
        let send_desc = &mut send_queue[index];

        // TODO: fix it
        return self.first_trans || (*send_desc).status & 1 != 0;
    }

    fn receive_available(&self) -> bool {
        // TODO map it in all cpu
        let mut current_addr = self.header;
        while current_addr < self.header + self.size {
            active_table().map_if_not_exists(current_addr, current_addr);
            current_addr = current_addr + PAGE_SIZE;
        }

        let e1000 =
            unsafe { slice::from_raw_parts_mut(self.header as *mut Volatile<u32>, self.size / 4) };
        let recv_queue_size = PAGE_SIZE / size_of::<E1000RecvDesc>();
        let mut recv_queue = unsafe {
            slice::from_raw_parts_mut(self.recv_page as *mut E1000RecvDesc, recv_queue_size)
        };
        let mut rdt = e1000[E1000_RDT].read();
        let index = (rdt as usize + 1) % recv_queue_size;
        let recv_desc = &mut recv_queue[index];
        return (*recv_desc).status & 1 != 0;
    }
}

impl NetDriver for E1000Driver {
    fn get_mac(&self) -> EthernetAddress {
        self.0.lock().mac
    }

    fn get_ifname(&self) -> String {
        format!("e1000")
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct E1000SendDesc {
    addr: u64,
    len: u16,
    cso: u8,
    cmd: u8,
    status: u8,
    css: u8,
    special: u8,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct E1000RecvDesc {
    addr: u64,
    len: u16,
    chksum: u16,
    status: u16,
    error: u8,
    special: u8,
}

pub struct E1000RxToken(E1000Driver);
pub struct E1000TxToken(E1000Driver);

impl<'a> phy::Device<'a> for E1000Driver {
    type RxToken = E1000RxToken;
    type TxToken = E1000TxToken;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        let driver = self.0.lock();
        if driver.transmit_available() && driver.receive_available() {
            // potential racing
            Some((E1000RxToken(self.clone()), E1000TxToken(self.clone())))
        } else {
            None
        }
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        let driver = self.0.lock();
        if driver.transmit_available() {
            Some(E1000TxToken(self.clone()))
        } else {
            None
        }
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1536;
        caps.max_burst_size = Some(1);
        caps
    }
}

impl phy::RxToken for E1000RxToken {
    fn consume<R, F>(self, _timestamp: Instant, f: F) -> Result<R>
    where
        F: FnOnce(&[u8]) -> Result<R>,
    {
        let data = {
            let mut driver = (self.0).0.lock();
            let e1000 = unsafe {
                slice::from_raw_parts_mut(driver.header as *mut Volatile<u32>, driver.size / 4)
            };
            let recv_queue_size = PAGE_SIZE / size_of::<E1000RecvDesc>();
            let mut recv_queue = unsafe {
                slice::from_raw_parts_mut(driver.recv_page as *mut E1000RecvDesc, recv_queue_size)
            };
            let mut rdt = e1000[E1000_RDT].read();
            let index = (rdt as usize + 1) % recv_queue_size;
            let recv_desc = &mut recv_queue[index];
            assert!(recv_desc.status & 1 != 0);
            let buffer = unsafe {
                slice::from_raw_parts(
                    driver.recv_buffers[index] as *const u8,
                    recv_desc.len as usize,
                )
            };

            recv_desc.status = recv_desc.status & !1;

            rdt = (rdt + 1) % recv_queue_size as u32;
            e1000[E1000_RDT].write(rdt);

            buffer
        };
        let result = f(&data);

        result
    }
}

impl phy::TxToken for E1000TxToken {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> Result<R>
    where
        F: FnOnce(&mut [u8]) -> Result<R>,
    {
        let mut buffer = [0u8; 2048];
        let result = f(&mut buffer[..len]);

        let mut driver = (self.0).0.lock();

        let e1000 = unsafe {
            slice::from_raw_parts_mut(driver.header as *mut Volatile<u32>, driver.size / 4)
        };
        let send_queue_size = PAGE_SIZE / size_of::<E1000SendDesc>();
        let mut send_queue = unsafe {
            slice::from_raw_parts_mut(driver.send_page as *mut E1000SendDesc, send_queue_size)
        };
        let mut tdt = e1000[E1000_TDT].read();

        let index_next = (tdt as usize + 1) % send_queue_size;
        let send_desc = &mut send_queue[index_next];
        assert!(driver.first_trans || send_desc.status & 1 != 0);

        let index = (tdt as usize) % send_queue_size;
        let send_desc = &mut send_queue[index];
        let target =
            unsafe { slice::from_raw_parts_mut(driver.send_buffers[index] as *mut u8, len) };
        target.copy_from_slice(&buffer[..len]);

        let buffer_page_pa = active_table()
            .get_entry(driver.send_buffers[index])
            .unwrap()
            .target();
        assert_eq!(buffer_page_pa, send_desc.addr as usize);
        send_desc.len = len as u16 + 4;
        // RS | EOP
        send_desc.cmd = (1 << 3) | (1 << 0);
        send_desc.status = 0;

        fence(Ordering::SeqCst);

        tdt = (tdt + 1) % send_queue_size as u32;
        e1000[E1000_TDT].write(tdt);

        fence(Ordering::SeqCst);

        // round
        if tdt == 0 {
            driver.first_trans = false;
        }

        result
    }
}

bitflags! {
    struct E1000Status : u32 {
        const FD = 1 << 0;
        const LU = 1 << 1;
        const TXOFF = 1 << 4;
        const TBIMODE = 1 << 5;
        const SPEED_100M = 1 << 6;
        const SPEED_1000M = 1 << 7;
        const ASDV_100M = 1 << 8;
        const ASDV_1000M = 1 << 9;
        const MTXCKOK = 1 << 10;
        const PCI66 = 1 << 11;
        const BUS64 = 1 << 12;
        const PCIX_MODE = 1 << 13;
        const GIO_MASTER_ENABLE = 1 << 19;
    }
}

// JudgeDuck-OS/kern/e1000.c
pub fn e1000_init(header: usize, size: usize) {
    info!("Probing e1000");
    assert_eq!(size_of::<E1000SendDesc>(), 16);
    assert_eq!(size_of::<E1000RecvDesc>(), 16);

    let send_page = unsafe {
        HEAP_ALLOCATOR.alloc_zeroed(Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap())
    } as usize;
    let recv_page = unsafe {
        HEAP_ALLOCATOR.alloc_zeroed(Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap())
    } as usize;
    let send_page_pa = active_table().get_entry(send_page).unwrap().target();
    let recv_page_pa = active_table().get_entry(recv_page).unwrap().target();
    let send_queue_size = PAGE_SIZE / size_of::<E1000SendDesc>();
    let recv_queue_size = PAGE_SIZE / size_of::<E1000RecvDesc>();
    let mut send_queue =
        unsafe { slice::from_raw_parts_mut(send_page as *mut E1000SendDesc, send_queue_size) };
    let mut recv_queue =
        unsafe { slice::from_raw_parts_mut(recv_page as *mut E1000RecvDesc, recv_queue_size) };
    // randomly generated
    let mac: [u8; 6] = [0x54, 0x51, 0x9F, 0x71, 0xC0, 0x3C];

    let mut driver = E1000 {
        header,
        size,
        mac: EthernetAddress::from_bytes(&mac),
        send_page,
        send_buffers: Vec::with_capacity(send_queue_size),
        recv_page,
        recv_buffers: Vec::with_capacity(recv_queue_size),
        first_trans: true,
    };

    let mut current_addr = header;
    while current_addr < header + size {
        active_table().map_if_not_exists(current_addr, current_addr);
        current_addr = current_addr + PAGE_SIZE;
    }

    let e1000 = unsafe { slice::from_raw_parts_mut(header as *mut Volatile<u32>, size / 4) };
    debug!(
        "status before setup: {:#?}",
        E1000Status::from_bits_truncate(e1000[E1000_STATUS].read())
    );

    e1000[E1000_TDBAL].write(send_page_pa as u32); // TDBAL
    e1000[E1000_TDBAH].write((send_page_pa >> 32) as u32); // TDBAH
    e1000[E1000_TDLEN].write(PAGE_SIZE as u32); // TDLEN
    e1000[E1000_TDH].write(0); // TDH
    e1000[E1000_TDT].write(0); // TDT

    for i in 0..send_queue_size {
        let buffer_page = unsafe {
            HEAP_ALLOCATOR.alloc_zeroed(Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap())
        } as usize;
        let buffer_page_pa = active_table().get_entry(buffer_page).unwrap().target();
        send_queue[i].addr = buffer_page_pa as u64;
        driver.send_buffers.push(buffer_page);
    }

    // EN | PSP | CT=0x10 | COLD=0x40
    e1000[E1000_TCTL].write((1 << 1) | (1 << 3) | (0x10 << 4) | (0x40 << 12)); // TCTL
    // IPGT=0xa | IPGR1=0x8 | IPGR2=0xc
    e1000[E1000_TIPG].write(0xa | (0x8 << 10) | (0xc << 20)); // TIPG

    let mut ral: u32 = 0;
    let mut rah: u32 = 0;
    for i in 0..4 {
        ral = ral | (mac[i] as u32) << (i * 8);
    }
    for i in 0..2 {
        rah = rah | (mac[i + 4] as u32) << (i * 8);
    }

    e1000[E1000_RAL].write(ral); // RAL
    // AV | AS=DA
    e1000[E1000_RAH].write(rah | (1 << 31)); // RAH

    // MTA
    for i in E1000_MTA..E1000_RAL {
        e1000[i].write(0);
    }
    e1000[E1000_IMS].write(0); // IMS

    e1000[E1000_RDBAL].write(recv_page_pa as u32); // RDBAL
    e1000[E1000_RDBAH].write((recv_page_pa >> 32) as u32); // RDBAH
    e1000[E1000_RDLEN].write(PAGE_SIZE as u32); // RDLEN
    e1000[E1000_RDH].write(0); // RDH
    e1000[E1000_RDT].write((recv_queue_size - 1) as u32); // RDT

    for i in 0..recv_queue_size {
        let buffer_page = unsafe {
            HEAP_ALLOCATOR.alloc_zeroed(Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap())
        } as usize;
        let buffer_page_pa = active_table().get_entry(buffer_page).unwrap().target();
        recv_queue[i].addr = buffer_page_pa as u64;
        driver.recv_buffers.push(buffer_page);
    }

    // EN | BAM | BSIZE=3 | BSEX | SECRC
    // BSIZE=3 | BSEX means buffer size = 4096
    e1000[E1000_RCTL].write((1 << 1) | (1 << 15) | (3 << 16) | (1 << 25) | (1 << 26)); // RCTL

    debug!(
        "status after setup: {:#?}",
        E1000Status::from_bits_truncate(e1000[E1000_STATUS].read())
    );

    let net_driver = E1000Driver(Arc::new(Mutex::new(driver)));

    DRIVERS.lock().push(Box::new(net_driver.clone()));
    NET_DRIVERS.lock().push(Box::new(net_driver));
}
