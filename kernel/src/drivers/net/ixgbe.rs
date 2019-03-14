//! Intel 10Gb Network Adapter 82599 i.e. ixgbe network driver

use alloc::alloc::{GlobalAlloc, Layout};
use alloc::format;
use alloc::prelude::*;
use alloc::sync::Arc;
use core::mem::size_of;
use core::slice;
use core::sync::atomic::{fence, Ordering};

use alloc::collections::BTreeMap;
use bitflags::*;
use log::*;
use rcore_memory::paging::PageTable;
use rcore_memory::PAGE_SIZE;
use smoltcp::iface::*;
use smoltcp::phy::{self, DeviceCapabilities};
use smoltcp::socket::*;
use smoltcp::time::Instant;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::*;
use smoltcp::Result;
use volatile::Volatile;

use crate::memory::active_table;
use crate::sync::SpinNoIrqLock as Mutex;
use crate::sync::{MutexGuard, SpinNoIrq};
use crate::HEAP_ALLOCATOR;

use super::super::{DeviceType, Driver, NetDriver, DRIVERS, NET_DRIVERS, SOCKET_ACTIVITY};

// At the beginning, all transmit descriptors have there status non-zero,
// so we need to track whether we are using the descriptor for the first time.
// When the descriptors wrap around, we set first_trans to false,
// and lookup status instead for checking whether it is empty.

pub struct IXGBE {
    header: usize,
    size: usize,
    mac: EthernetAddress,
    send_page: usize,
    send_buffers: Vec<usize>,
    recv_page: usize,
    recv_buffers: Vec<usize>,
    first_trans: bool,
}

#[derive(Clone)]
pub struct IXGBEDriver(Arc<Mutex<IXGBE>>);

const IXGBE_CTRL: usize = 0x00000 / 4;
const IXGBE_STATUS: usize = 0x00008 / 4;
const IXGBE_CTRL_EXT: usize = 0x00018 / 4;
const IXGBE_EICR: usize = 0x00800 / 4;
const IXGBE_EIMS: usize = 0x00880 / 4;
const IXGBE_EIMC: usize = 0x00888 / 4;
const IXGBE_IVAR: usize = 0x00900 / 4;
const IXGBE_EIMC1: usize = 0x00A90 / 4;
const IXGBE_EIMC2: usize = 0x00A91 / 4;
const IXGBE_RDBAL: usize = 0x01000 / 4;
const IXGBE_RDBAH: usize = 0x01004 / 4;
const IXGBE_RDLEN: usize = 0x01008 / 4;
const IXGBE_DCA_RXCTRL: usize = 0x0100C / 4;
const IXGBE_RDH: usize = 0x01010 / 4;
const IXGBE_SRRCTL: usize = 0x01014 / 4;
const IXGBE_RDT: usize = 0x01018 / 4;
const IXGBE_RXDCTL: usize = 0x01028 / 4;
const IXGBE_RDRXCTL: usize = 0x02F00 / 4;
const IXGBE_RXCTRL: usize = 0x03000 / 4;
const IXGBE_FCTTV: usize = 0x03200 / 4;
const IXGBE_FCTTV_END: usize = 0x03210 / 4;
const IXGBE_FCRTL: usize = 0x03220 / 4;
const IXGBE_FCRTL_END: usize = 0x03240 / 4;
const IXGBE_FCRTH: usize = 0x03260 / 4;
const IXGBE_FCRTH_END: usize = 0x03280 / 4;
const IXGBE_FCRTV: usize = 0x032A0 / 4;
const IXGBE_FCCFG: usize = 0x03D00 / 4;
const IXGBE_AUTOC: usize = 0x042A0 / 4;
const IXGBE_LINKS: usize = 0x042A4 / 4;
const IXGBE_AUTOC2: usize = 0x04324 / 4;
const IXGBE_DMATXCTL: usize = 0x04A80 / 4;
const IXGBE_FCTRL: usize = 0x05080 / 4;
const IXGBE_MTA: usize = 0x05200 / 4;
const IXGBE_MTA_END: usize = 0x05400 / 4;
const IXGBE_TDBAL: usize = 0x06000 / 4;
const IXGBE_TDBAH: usize = 0x06004 / 4;
const IXGBE_TDLEN: usize = 0x06008 / 4;
const IXGBE_TDH: usize = 0x06010 / 4;
const IXGBE_TDT: usize = 0x06018 / 4;
const IXGBE_TXDCTL: usize = 0x06028 / 4;
const IXGBE_SECRXCTRL: usize = 0x08D00 / 4;
const IXGBE_SECRXSTAT: usize = 0x08D04 / 4;
const IXGBE_VFTA: usize = 0x0A000 / 4;
const IXGBE_VFTA_END: usize = 0x0A200 / 4;
const IXGBE_RAL: usize = 0x0A200 / 4;
const IXGBE_RAH: usize = 0x0A204 / 4;
const IXGBE_MPSAR: usize = 0x0A600 / 4;
const IXGBE_MPSAR_END: usize = 0x0A800 / 4;
const IXGBE_PFVLVF: usize = 0x0F100 / 4;
const IXGBE_PFVLVF_END: usize = 0x0F200 / 4;
const IXGBE_PFVLVFB: usize = 0x0F200 / 4;
const IXGBE_PFVLVFB_END: usize = 0x0F400 / 4;
const IXGBE_PFUTA: usize = 0x0F400 / 4;
const IXGBE_PFUTA_END: usize = 0x0F600 / 4;
const IXGBE_EEC: usize = 0x10010 / 4;

pub struct IXGBEInterface {
    iface: Mutex<EthernetInterface<'static, 'static, 'static, IXGBEDriver>>,
    driver: IXGBEDriver,
    sockets: Mutex<SocketSet<'static, 'static, 'static>>,
}

impl Driver for IXGBEInterface {
    fn try_handle_interrupt(&self) -> bool {
        let irq = {
            let driver = self.driver.0.lock();

            if let None = active_table().get_entry(driver.header) {
                let mut current_addr = driver.header;
                while current_addr < driver.header + driver.size {
                    active_table().map_if_not_exists(current_addr, current_addr);
                    current_addr = current_addr + PAGE_SIZE;
                }
            }

            let ixgbe = unsafe {
                slice::from_raw_parts_mut(driver.header as *mut Volatile<u32>, driver.size / 4)
            };

            let icr = ixgbe[IXGBE_EICR].read();
            if icr != 0 {
                // clear it
                ixgbe[IXGBE_EICR].write(icr);
                true
            } else {
                false
            }
        };

        if irq {
            let timestamp = Instant::from_millis(crate::trap::uptime_msec() as i64);
            let mut sockets = self.sockets.lock();
            match self.iface.lock().poll(&mut sockets, timestamp) {
                Ok(_) => {
                    SOCKET_ACTIVITY.notify_all();
                }
                Err(err) => {
                    debug!("poll got err {}", err);
                }
            }
        }

        return irq;
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Net
    }
}

impl NetDriver for IXGBEInterface {
    fn get_mac(&self) -> EthernetAddress {
        self.iface.lock().ethernet_addr()
    }

    fn get_ifname(&self) -> String {
        format!("ixgbe")
    }

    fn ipv4_address(&self) -> Option<Ipv4Address> {
        self.iface.lock().ipv4_address()
    }

    fn sockets(&self) -> MutexGuard<SocketSet<'static, 'static, 'static>, SpinNoIrq> {
        self.sockets.lock()
    }

    fn poll(&self) {
        let timestamp = Instant::from_millis(crate::trap::uptime_msec() as i64);
        let mut sockets = self.sockets.lock();
        match self.iface.lock().poll(&mut sockets, timestamp) {
            Ok(_) => {
                SOCKET_ACTIVITY.notify_all();
            }
            Err(err) => {
                debug!("poll got err {}", err);
            }
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct IXGBESendDesc {
    addr: u64,
    len: u16,
    cso: u8,
    cmd: u8,
    status: u8,
    css: u8,
    vlan: u16,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct IXGBERecvDesc {
    addr: u64,
    len: u16,
    frag_chksum: u16,
    status_error: u16,
    vlan_tag: u16,
}

pub struct IXGBERxToken(Vec<u8>);
pub struct IXGBETxToken(IXGBEDriver);

impl<'a> phy::Device<'a> for IXGBEDriver {
    type RxToken = IXGBERxToken;
    type TxToken = IXGBETxToken;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        let driver = self.0.lock();

        if let None = active_table().get_entry(driver.header) {
            let mut current_addr = driver.header;
            while current_addr < driver.header + driver.size {
                active_table().map_if_not_exists(current_addr, current_addr);
                current_addr = current_addr + PAGE_SIZE;
            }
        }

        let ixgbe = unsafe {
            slice::from_raw_parts_mut(driver.header as *mut Volatile<u32>, driver.size / 4)
        };

        let send_queue_size = PAGE_SIZE / size_of::<IXGBESendDesc>();
        let send_queue = unsafe {
            slice::from_raw_parts_mut(driver.send_page as *mut IXGBESendDesc, send_queue_size)
        };
        let tdt = ixgbe[IXGBE_TDT].read();
        let index = (tdt as usize) % send_queue_size;
        let send_desc = &mut send_queue[index];

        let recv_queue_size = PAGE_SIZE / size_of::<IXGBERecvDesc>();
        let mut recv_queue = unsafe {
            slice::from_raw_parts_mut(driver.recv_page as *mut IXGBERecvDesc, recv_queue_size)
        };
        let mut rdt = ixgbe[IXGBE_RDT].read();
        let index = (rdt as usize + 1) % recv_queue_size;
        let recv_desc = &mut recv_queue[index];

        let transmit_avail = driver.first_trans || (*send_desc).status & 1 != 0;
        // Ignore packet spanning multiple descriptor
        let receive_avail = (*recv_desc).status_error & 1 != 0;

        if transmit_avail && receive_avail {
            let buffer = unsafe {
                slice::from_raw_parts(
                    driver.recv_buffers[index] as *const u8,
                    recv_desc.len as usize,
                )
            };

            recv_desc.status_error = recv_desc.status_error & !1;

            rdt = (rdt + 1) % recv_queue_size as u32;
            ixgbe[IXGBE_RDT].write(rdt);

            Some((IXGBERxToken(buffer.to_vec()), IXGBETxToken(self.clone())))
        } else {
            None
        }
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        let driver = self.0.lock();

        if let None = active_table().get_entry(driver.header) {
            let mut current_addr = driver.header;
            while current_addr < driver.header + driver.size {
                active_table().map_if_not_exists(current_addr, current_addr);
                current_addr = current_addr + PAGE_SIZE;
            }
        }

        let ixgbe = unsafe {
            slice::from_raw_parts_mut(driver.header as *mut Volatile<u32>, driver.size / 4)
        };

        let send_queue_size = PAGE_SIZE / size_of::<IXGBESendDesc>();
        let send_queue = unsafe {
            slice::from_raw_parts_mut(driver.send_page as *mut IXGBESendDesc, send_queue_size)
        };
        let tdt = ixgbe[IXGBE_TDT].read();
        let index = (tdt as usize) % send_queue_size;
        let send_desc = &mut send_queue[index];
        let transmit_avail = driver.first_trans || (*send_desc).status & 1 != 0;
        if transmit_avail {
            Some(IXGBETxToken(self.clone()))
        } else {
            None
        }
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1536;
        caps.max_burst_size = Some(64);
        caps
    }
}

impl phy::RxToken for IXGBERxToken {
    fn consume<R, F>(self, _timestamp: Instant, f: F) -> Result<R>
    where
        F: FnOnce(&[u8]) -> Result<R>,
    {
        f(&self.0)
    }
}

impl phy::TxToken for IXGBETxToken {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> Result<R>
    where
        F: FnOnce(&mut [u8]) -> Result<R>,
    {
        let mut buffer = [0u8; PAGE_SIZE];
        let result = f(&mut buffer[..len]);

        let mut driver = (self.0).0.lock();

        let ixgbe = unsafe {
            slice::from_raw_parts_mut(driver.header as *mut Volatile<u32>, driver.size / 4)
        };
        let send_queue_size = PAGE_SIZE / size_of::<IXGBESendDesc>();
        let mut send_queue = unsafe {
            slice::from_raw_parts_mut(driver.send_page as *mut IXGBESendDesc, send_queue_size)
        };
        let mut tdt = ixgbe[IXGBE_TDT].read();

        let index = (tdt as usize) % send_queue_size;
        let send_desc = &mut send_queue[index];
        assert!(driver.first_trans || send_desc.status & 1 != 0);

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
        ixgbe[IXGBE_TDT].write(tdt);

        fence(Ordering::SeqCst);

        // round
        if tdt == 0 {
            driver.first_trans = false;
        }

        result
    }
}

bitflags! {
    struct IXGBEStatus : u32 {
        const LANID0 = 1 << 2;
        const LABID1 = 1 << 3;
        const LINK_UP = 1 << 7;
        const NUM_VFS1 = 1 << 10;
        const NUM_VFS2 = 1 << 11;
        const NUM_VFS4 = 1 << 12;
        const NUM_VFS8 = 1 << 13;
        const NUM_VFS16 = 1 << 14;
        const NUM_VFS32 = 1 << 15;
        const NUM_VFS64 = 1 << 16;
        const IOV = 1 << 18;
        const PCIE_MASTER_ENABLE = 1 << 19;
    }
}

pub fn ixgbe_init(header: usize, size: usize) {
    info!("Probing ixgbe");
    assert_eq!(size_of::<IXGBESendDesc>(), 16);
    assert_eq!(size_of::<IXGBERecvDesc>(), 16);

    let send_page = unsafe {
        HEAP_ALLOCATOR.alloc_zeroed(Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap())
    } as usize;
    let recv_page = unsafe {
        HEAP_ALLOCATOR.alloc_zeroed(Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap())
    } as usize;
    let send_page_pa = active_table().get_entry(send_page).unwrap().target();
    let recv_page_pa = active_table().get_entry(recv_page).unwrap().target();
    let send_queue_size = PAGE_SIZE / size_of::<IXGBESendDesc>();
    let recv_queue_size = PAGE_SIZE / size_of::<IXGBERecvDesc>();
    let mut send_queue =
        unsafe { slice::from_raw_parts_mut(send_page as *mut IXGBESendDesc, send_queue_size) };
    let mut recv_queue =
        unsafe { slice::from_raw_parts_mut(recv_page as *mut IXGBERecvDesc, recv_queue_size) };
    // randomly generated
    let mac: [u8; 6] = [0x54, 0x51, 0x9F, 0x71, 0xC0, 0x3C];

    let mut current_addr = header;
    while current_addr < header + size {
        active_table().map_if_not_exists(current_addr, current_addr);
        current_addr = current_addr + PAGE_SIZE;
    }

    let ixgbe = unsafe { slice::from_raw_parts_mut(header as *mut Volatile<u32>, size / 4) };
    debug!(
        "status before setup: {:#?}",
        IXGBEStatus::from_bits_truncate(ixgbe[IXGBE_STATUS].read())
    );

    // 4.6.3 Initialization Sequence

    // 4.6.3.1 Interrupts During Initialization
    // 1. Disable interrupts.
    // mask all interrupts
    ixgbe[IXGBE_EIMC].write(!0);
    ixgbe[IXGBE_EIMC1].write(!0);
    ixgbe[IXGBE_EIMC2].write(!0);

    // 2. Issue a global reset.
    // reset: LRST | RST
    ixgbe[IXGBE_CTRL].write(1 << 3 | 1 << 26);
    while ixgbe[IXGBE_CTRL].read() & (1 << 3 | 1 << 26) != 0 {}

    // 3. Disable interrupts (again).
    // mask all interrupts
    ixgbe[IXGBE_EIMC].write(!0);
    ixgbe[IXGBE_EIMC1].write(!0);
    ixgbe[IXGBE_EIMC2].write(!0);

    // 4.6.3.2 Global Reset and General Configuration
    // no flow control
    for reg in (IXGBE_FCTTV..IXGBE_FCTTV_END).step_by(4) {
        ixgbe[reg].write(0);
    }
    for reg in (IXGBE_FCRTL..IXGBE_FCRTL_END).step_by(4) {
        ixgbe[reg].write(0);
    }
    for reg in (IXGBE_FCRTH..IXGBE_FCRTH_END).step_by(4) {
        ixgbe[reg].write(0);
    }
    ixgbe[IXGBE_FCRTV].write(0);
    ixgbe[IXGBE_FCCFG].write(0);

    // Auto-Read Done
    while ixgbe[IXGBE_EEC].read() & (1 << 9) == 0 {}

    // DMA Init Done
    while ixgbe[IXGBE_RDRXCTL].read() & (1 << 3) == 0 {}

    let mut driver = IXGBE {
        header,
        size,
        mac: EthernetAddress::from_bytes(&mac),
        send_page,
        send_buffers: Vec::with_capacity(send_queue_size),
        recv_page,
        recv_buffers: Vec::with_capacity(recv_queue_size),
        first_trans: true,
    };

    // BAM, Accept Broadcast packets
    ixgbe[IXGBE_FCTRL].write(ixgbe[IXGBE_FCTRL].read() | (1 << 10));

    // 4.6.7 Receive Initialization

    // Receive Address (RAL[n] and RAH[n]) for used addresses.
    // Read MAC Address
    let ral = ixgbe[IXGBE_RAL].read();
    let rah = ixgbe[IXGBE_RAH].read();
    let mac: [u8; 6] = [
        ral as u8,
        (ral >> 8) as u8,
        (ral >> 16) as u8,
        (ral >> 24) as u8,
        rah as u8,
        (rah >> 8) as u8,
    ];
    debug!("mac {:x?}", mac);

    // Unicast Table Array (PFUTA).
    for i in IXGBE_PFUTA..IXGBE_PFUTA_END {
        ixgbe[i].write(0);
    }
    // VLAN Filter Table Array (VFTA[n]).
    for i in IXGBE_VFTA..IXGBE_VFTA_END {
        ixgbe[i].write(0);
    }
    // VLAN Pool Filter (PFVLVF[n]).
    for i in IXGBE_PFVLVF..IXGBE_PFVLVF_END {
        ixgbe[i].write(0);
    }
    // MAC Pool Select Array (MPSAR[n]).
    for i in IXGBE_MPSAR..IXGBE_MPSAR_END {
        ixgbe[i].write(0);
    }
    // VLAN Pool Filter Bitmap (PFVLVFB[n]).
    for i in IXGBE_PFVLVFB..IXGBE_PFVLVFB_END {
        ixgbe[i].write(0);
    }
    // Set up the Multicast Table Array (MTA) registers. This entire table should be zeroed and only the desired multicast addresses should be permitted (by writing 0x1 to the corresponding bit location).
    for i in IXGBE_MTA..IXGBE_MTA_END {
        ixgbe[i].write(0);
    }

    // Setup receive queue 0
    // The following steps should be done once per transmit queue:
    // 2. Receive buffers of appropriate size should be allocated and pointers to these buffers should be stored in the descriptor ring.
    for i in 0..recv_queue_size {
        let buffer_page = unsafe {
            HEAP_ALLOCATOR.alloc_zeroed(Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap())
        } as usize;
        let buffer_page_pa = active_table().get_entry(buffer_page).unwrap().target();
        recv_queue[i].addr = buffer_page_pa as u64;
        driver.recv_buffers.push(buffer_page);
    }

    // 3. Program the descriptor base address with the address of the region (registers RDBAL, RDBAH).
    ixgbe[IXGBE_RDBAL].write(recv_page_pa as u32); // RDBAL
    ixgbe[IXGBE_RDBAH].write((recv_page_pa >> 32) as u32); // RDBAH

    // 4. Set the length register to the size of the descriptor ring (register RDLEN).
    ixgbe[IXGBE_RDLEN].write(PAGE_SIZE as u32); // RDLEN

    // 5. Program SRRCTL associated with this queue according to the size of the buffers and the required header control.
    // Legacy descriptor, default SRRCTL is ok

    ixgbe[IXGBE_RDH].write(0); // RDH

    // 8. Program RXDCTL with appropriate values including the queue Enable bit. Note that packets directed to a disabled queue are dropped.
    ixgbe[IXGBE_RXDCTL].write(ixgbe[IXGBE_RXDCTL].read() | (1 << 25)); // enable queue

    // 9. Poll the RXDCTL register until the Enable bit is set. The tail should not be bumped before this bit was read as 1b.
    while ixgbe[IXGBE_RXDCTL].read() | (1 << 25) == 0 {} // wait for it

    // 10. Bump the tail pointer (RDT) to enable descriptors fetching by setting it to the ring length minus one.
    ixgbe[IXGBE_RDT].write((recv_queue_size - 1) as u32); // RDT

    // all queues are setup
    // 11. Enable the receive path by setting RXCTRL.RXEN. This should be done only after all other settings are done following the steps below.
    // Halt the receive data path by setting SECRXCTRL.RX_DIS bit.
    ixgbe[IXGBE_SECRXCTRL].write(ixgbe[IXGBE_SECRXCTRL].read() | (1 << 1));
    // Wait for the data paths to be emptied by HW. Poll the SECRXSTAT.SECRX_RDY bit until it is asserted by HW.
    while ixgbe[IXGBE_SECRXSTAT].read() & (1 << 0) == 0 {} // poll
                                                           // Set RXCTRL.RXEN
                                                           // enable the queue
    ixgbe[IXGBE_RXCTRL].write(ixgbe[IXGBE_RXCTRL].read() | (1 << 0));
    // Clear the SECRXCTRL.SECRX_DIS bits to enable receive data path
    ixgbe[IXGBE_SECRXCTRL].write(ixgbe[IXGBE_SECRXCTRL].read() & !(1 << 1));

    // Set bit 16 of the CTRL_EXT register and clear bit 12 of the DCA_RXCTRL[n] register[n].
    ixgbe[IXGBE_CTRL_EXT].write(ixgbe[IXGBE_CTRL_EXT].read() | (1 << 16));
    ixgbe[IXGBE_DCA_RXCTRL].write(ixgbe[IXGBE_DCA_RXCTRL].read() & !(1 << 12));

    // 4.6.8 Transmit Initialization

    // The following steps should be done once per transmit queue:
    // 1. Allocate a region of memory for the transmit descriptor list.
    for i in 0..send_queue_size {
        let buffer_page = unsafe {
            HEAP_ALLOCATOR.alloc_zeroed(Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap())
        } as usize;
        let buffer_page_pa = active_table().get_entry(buffer_page).unwrap().target();
        send_queue[i].addr = buffer_page_pa as u64;
        driver.send_buffers.push(buffer_page);
    }

    // 2. Program the descriptor base address with the address of the region (TDBAL, TDBAH).
    ixgbe[IXGBE_TDBAL].write(send_page_pa as u32); // TDBAL
    ixgbe[IXGBE_TDBAH].write((send_page_pa >> 32) as u32); // TDBAH

    // 3. Set the length register to the size of the descriptor ring (TDLEN).
    ixgbe[IXGBE_TDLEN].write(PAGE_SIZE as u32); // TDLEN
    ixgbe[IXGBE_TDH].write(0); // TDH
    ixgbe[IXGBE_TDT].write(0); // TDT

    // 6. Enable transmit path by setting DMATXCTL.TE. This step should be executed only for the first enabled transmit queue and does not need to be repeated for any following queues.
    ixgbe[IXGBE_DMATXCTL].write(ixgbe[IXGBE_DMATXCTL].read() | 1 << 0);

    // 7. Enable the queue using TXDCTL.ENABLE. Poll the TXDCTL register until the Enable bit is set.
    ixgbe[IXGBE_TXDCTL].write(ixgbe[IXGBE_TXDCTL].read() | 1 << 25);
    while ixgbe[IXGBE_TXDCTL].read() & (1 << 25) == 0 {}

    // Enable interrupts
    // map Rx0 and Tx0 to interrupt 0
    ixgbe[IXGBE_IVAR].write(0b00000000_00000000_10000000_10000000);

    // clear all interrupt
    ixgbe[IXGBE_EICR].write(!0);
    // unmask tx/rx interrupts
    ixgbe[IXGBE_EIMS].write(1 << 0);

    debug!(
        "status after setup: {:#?}",
        IXGBEStatus::from_bits_truncate(ixgbe[IXGBE_STATUS].read())
    );

    let net_driver = IXGBEDriver(Arc::new(Mutex::new(driver)));

    let ethernet_addr = EthernetAddress::from_bytes(&mac);
    let ip_addrs = [IpCidr::new(IpAddress::v4(10, 0, 0, 2), 24)];
    let neighbor_cache = NeighborCache::new(BTreeMap::new());
    let iface = EthernetInterfaceBuilder::new(net_driver.clone())
        .ethernet_addr(ethernet_addr)
        .ip_addrs(ip_addrs)
        .neighbor_cache(neighbor_cache)
        .finalize();

    let ixgbe_iface = IXGBEInterface {
        iface: Mutex::new(iface),
        sockets: Mutex::new(SocketSet::new(vec![])),
        driver: net_driver.clone(),
    };

    let driver = Arc::new(ixgbe_iface);
    DRIVERS.write().push(driver.clone());
    NET_DRIVERS.write().push(driver);
}
