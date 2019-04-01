//! Driver for AHCI
//!
//! Spec: https://www.intel.com/content/dam/www/public/us/en/documents/technical-specifications/serial-ata-ahci-spec-rev1-3-1.pdf

use alloc::alloc::{alloc_zeroed, Layout};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::mem::size_of;
use core::slice;
use core::sync::atomic::spin_loop_hint;

use bit_field::*;
use bitflags::*;
use log::*;
use rcore_fs::dev::BlockDevice;
use volatile::Volatile;

use rcore_memory::paging::PageTable;
use rcore_memory::{PhysAddr, VirtAddr, PAGE_SIZE};

use crate::drivers::BlockDriver;
use crate::memory::active_table;
use crate::sync::SpinNoIrqLock as Mutex;

use super::super::{DeviceType, Driver, BLK_DRIVERS, DRIVERS};

pub struct AHCI {
    header: usize,
    size: usize,
    received_fis: &'static mut AHCIReceivedFIS,
    cmd_list: &'static mut [AHCICommandHeader],
    cmd_table: &'static mut AHCICommandTable,
    data: &'static mut [u8],
    port: &'static mut AHCIPort,
}

pub struct AHCIDriver(Mutex<AHCI>);

/// AHCI Generic Host Control (3.1)
#[repr(C)]
pub struct AHCIGHC {
    /// Host capability
    capability: Volatile<AHCICap>,
    /// Global host control
    global_host_control: Volatile<u32>,
    /// Interrupt status
    interrupt_status: Volatile<u32>,
    /// Port implemented
    port_implemented: Volatile<u32>,
    /// Version
    version: Volatile<u32>,
    /// Command completion coalescing control
    ccc_control: Volatile<u32>,
    /// Command completion coalescing ports
    ccc_ports: Volatile<u32>,
    /// Enclosure management location
    em_location: Volatile<u32>,
    /// Enclosure management control
    em_control: Volatile<u32>,
    /// Host capabilities extended
    capabilities2: Volatile<u32>,
    /// BIOS/OS handoff control and status
    bios_os_handoff_control: Volatile<u32>,
}

bitflags! {
    struct AHCICap : u32 {
        const S64A = 1 << 31;
        const SNCQ = 1 << 30;
        const SSNTF = 1 << 29;
        const SMPS = 1 << 28;
        const SSS = 1 << 27;
        const SALP = 1 << 26;
        const SAL = 1 << 25;
        const SCLO = 1 << 24;
        const ISS_GEN_1 = 1 << 20;
        const ISS_GEN_2 = 2 << 20;
        const ISS_GEN_3 = 3 << 20;
        const SAM = 1 << 18;
        const SPM = 1 << 17;
        const FBSS = 1 << 16;
        const PMD = 1 << 15;
        const SSC = 1 << 14;
        const PSC = 1 << 13;
        const CCCS = 1 << 7;
        const EMS = 1 << 6;
        const SXS = 1 << 5;
        // number of ports = 1
        const NUM_MASK = 0b11111;
    }
}

impl AHCIGHC {
    fn enable(&mut self) {
        self.global_host_control.update(|v| {
            v.set_bit(13, true);
        });
    }
    fn num_ports(&self) -> usize {
        (self.capability.read() & AHCICap::NUM_MASK).bits() as usize + 1
    }
    fn has_port(&self, port_num: usize) -> bool {
        self.port_implemented.read().get_bit(port_num)
    }
}

/// AHCI Port Registers (3.3) (one set per port)
#[repr(C)]
pub struct AHCIPort {
    command_list_base_address: Volatile<u64>,
    fis_base_address: Volatile<u64>,
    interrupt_status: Volatile<u32>,
    interrupt_enable: Volatile<u32>,
    command: Volatile<u32>,
    reserved: Volatile<u32>,
    task_file_data: Volatile<u32>,
    signature: Volatile<u32>,
    sata_status: Volatile<u32>,
    sata_control: Volatile<u32>,
    sata_error: Volatile<u32>,
    sata_active: Volatile<u32>,
    command_issue: Volatile<u32>,
    sata_notification: Volatile<u32>,
    fis_based_switch_control: Volatile<u32>,
}

impl AHCIPort {
    fn spin_on_slot(&mut self, slot: usize) {
        loop {
            let ci = self.command_issue.read();
            if !ci.get_bit(slot) {
                break;
            }
            spin_loop_hint();
        }
    }
    fn issue_command(&mut self, slot: usize) {
        assert!(slot < 32);
        self.command_issue.write(1 << (slot as u32));
    }
}

/// AHCI Received FIS Structure (4.2.1)
#[repr(C)]
pub struct AHCIReceivedFIS {
    dma: [u8; 0x20],
    pio: [u8; 0x20],
    d2h: [u8; 0x18],
    sdbfis: [u8; 0x8],
    ufis: [u8; 0x40],
    reserved: [u8; 0x60],
}

/// # AHCI Command List Structure (4.2.2)
///
/// Host sends commands to the device through Command List.
///
/// Command List consists of 1 to 32 command headers, each one is called a slot.
///
/// Each command header describes an ATA or ATAPI command, including a
/// Command FIS, an ATAPI command buffer and a bunch of Physical Region
/// Descriptor Tables specifying the data payload address and size.
///
/// https://wiki.osdev.org/images/e/e8/Command_list.jpg
#[repr(C)]
pub struct AHCICommandHeader {
    ///
    flags: CommandHeaderFlags,
    /// Physical region descriptor table length in entries
    prdt_length: u16,
    /// Physical region descriptor byte count transferred
    prd_byte_count: u32,
    /// Command table descriptor base address
    command_table_base_address: u64,
    /// Reserved
    reserved: [u32; 4],
}

bitflags! {
    pub struct CommandHeaderFlags: u16 {
        /// Command FIS length in DWORDS, 2 ~ 16
        const CFL_MASK = 0b11111;
        /// ATAPI
        const ATAPI = 1 << 5;
        /// Write, 1: H2D, 0: D2H
        const WRITE = 1 << 6;
        /// Prefetchable
        const PREFETCHABLE = 1 << 7;
        /// Reset
        const RESET = 1 << 8;
        /// BIST
        const BIST = 1 << 9;
        /// Clear busy upon R_OK
        const CLEAR = 1 << 10;
        /// Port multiplier port
        const PORT_MULTIPLIER_PORT_MASK = 0b1111 << 12;
    }
}

/// AHCI Command Table (4.2.3)
#[repr(C)]
pub struct AHCICommandTable {
    /// Command FIS
    cfis: SATAFISRegH2D,
    /// ATAPI command, 12 or 16 bytes
    acmd: [u8; 16],
    /// Reserved
    reserved: [u8; 48],
    /// Physical region descriptor table entries, 0 ~ 65535
    prdt: [AHCIPrdtEntry; 1],
}

/// Physical region descriptor table entry
#[repr(C)]
pub struct AHCIPrdtEntry {
    /// Data base address
    data_base_address: u64,
    /// Reserved
    reserved: u32,
    /// Bit 21-0: Byte count, 4M max
    /// Bit 31:   Interrupt on completion
    dbc_i: u32,
}

const FIS_REG_H2D: u8 = 0x27;

const CMD_READ_DMA_EXT: u8 = 0x25;
const CMD_WRITE_DMA_EXT: u8 = 0x35;
const CMD_IDENTIFY_DEVICE: u8 = 0xec;

/// SATA Register FIS - Host to Device
///
/// https://wiki.osdev.org/AHCI Figure 5-2
#[repr(C)]
pub struct SATAFISRegH2D {
    fis_type: u8,
    cflags: u8,
    command: u8,
    feature_lo: u8,

    lba_0: u8, // LBA 7:0
    lba_1: u8, // LBA 15:8
    lba_2: u8, // LBA 23:16
    dev_head: u8,

    lba_3: u8, // LBA 31:24
    lba_4: u8, // LBA 39:32
    lba_5: u8, // LBA 47:40
    feature_hi: u8,

    sector_count: u16,
    reserved: u8,
    control: u8,

    _padding: [u8; 48],
}

impl SATAFISRegH2D {
    fn set_lba(&mut self, lba: u64) {
        self.lba_0 = (lba >> 0) as u8;
        self.lba_1 = (lba >> 8) as u8;
        self.lba_2 = (lba >> 16) as u8;
        self.lba_3 = (lba >> 24) as u8;
        self.lba_4 = (lba >> 32) as u8;
        self.lba_5 = (lba >> 40) as u8;
    }
}

/// IDENTIFY DEVICE data
///
/// ATA8-ACS Table 29
#[repr(C)]
pub struct ATAIdentifyPacket {
    _1: [u16; 10],
    serial: [u8; 20], // words 10-19
    _2: [u16; 3],
    firmware: [u8; 8], // words 23-26
    model: [u8; 40],   // words 27-46
    _3: [u16; 13],
    lba_sectors: u32, // words 60-61
    _4: [u16; 38],
    lba48_sectors: u64, // words 100-103
}

impl AHCI {
    fn read_block(&mut self, block_id: usize, buf: &mut [u8]) -> usize {
        self.cmd_list[0].flags = CommandHeaderFlags::empty();

        let fis = &mut self.cmd_table.cfis;
        // Register FIS from HBA to device
        fis.fis_type = FIS_REG_H2D;
        fis.cflags = 1 << 7;
        // 7.25 READ DMA EXT - 25h, DMA
        fis.command = CMD_READ_DMA_EXT;
        fis.sector_count = 1;
        fis.dev_head = 0x40; // LBA
        fis.control = 0x80; // LBA48
        fis.set_lba(block_id as u64);

        self.port.issue_command(0);
        self.port.spin_on_slot(0);

        let len = buf.len().min(BLOCK_SIZE);
        buf[..len].clone_from_slice(&self.data[0..len]);
        len
    }

    fn write_block(&mut self, block_id: usize, buf: &[u8]) -> usize {
        self.cmd_list[0].flags = CommandHeaderFlags::WRITE; // device write

        let len = buf.len().min(BLOCK_SIZE);
        self.data[0..len].clone_from_slice(&buf[..len]);

        let fis = &mut self.cmd_table.cfis;
        // Register FIS from HBA to device
        fis.fis_type = FIS_REG_H2D;
        fis.cflags = 1 << 7;
        // ATA8-ACS
        // 7.63 WRITE DMA EXT - 35h, DMA
        fis.command = CMD_WRITE_DMA_EXT;
        fis.sector_count = 1;
        fis.dev_head = 0x40; // LBA
        fis.control = 0x80; // LBA48
        fis.set_lba(block_id as u64);

        self.port.issue_command(0);
        self.port.spin_on_slot(0);

        len
    }
}

impl Driver for AHCIDriver {
    fn try_handle_interrupt(&self, _irq: Option<u32>) -> bool {
        false
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Block
    }

    fn get_id(&self) -> String {
        format!("ahci")
    }

    fn read_block(&self, block_id: usize, buf: &mut [u8]) -> bool {
        let mut driver = self.0.lock();
        driver.read_block(block_id, buf);
        true
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) -> bool {
        if buf.len() < BLOCK_SIZE {
            return false;
        }
        let mut driver = self.0.lock();
        driver.write_block(block_id, buf);
        true
    }
}

const BLOCK_SIZE: usize = 512;

fn from_ata_string(data: &[u8]) -> String {
    let mut swapped_data = Vec::new();
    assert_eq!(data.len() % 2, 0);
    for i in (0..data.len()).step_by(2) {
        swapped_data.push(data[i + 1]);
        swapped_data.push(data[i]);
    }
    return String::from_utf8(swapped_data).unwrap();
}

/// Allocate consequent physical frames for DMA
fn alloc_dma(page_num: usize) -> (VirtAddr, PhysAddr) {
    let layout = Layout::from_size_align(PAGE_SIZE * page_num, PAGE_SIZE).unwrap();
    let vaddr = unsafe { alloc_zeroed(layout) } as usize;
    let paddr = active_table().get_entry(vaddr).unwrap().target();
    (vaddr, paddr)
}

pub fn ahci_init(irq: Option<u32>, header: usize, size: usize) -> Arc<AHCIDriver> {
    let ghc = unsafe { &mut *(header as *mut AHCIGHC) };

    ghc.enable();

    for port_num in 0..ghc.num_ports() {
        if ghc.has_port(port_num) {
            let addr = header + 0x100 + 0x80 * port_num;
            let port = unsafe { &mut *(addr as *mut AHCIPort) };

            // SSTS IPM Active
            if port.sata_status.read().get_bits(8..12) != 1 {
                continue;
            }

            // SSTS DET Present
            if port.sata_status.read().get_bits(0..4) != 3 {
                continue;
            }

            debug!("probing port {}", port_num);
            // Disable Port First
            port.command.update(|c| {
                c.set_bit(4, false);
                c.set_bit(0, false);
            });

            let (rfis_va, rfis_pa) = alloc_dma(1);
            let (cmd_list_va, cmd_list_pa) = alloc_dma(1);
            let (cmd_table_va, cmd_table_pa) = alloc_dma(1);
            let (data_va, data_pa) = alloc_dma(1);

            let received_fis = unsafe { &mut *(rfis_va as *mut AHCIReceivedFIS) };
            let cmd_list = unsafe {
                slice::from_raw_parts_mut(
                    cmd_list_va as *mut AHCICommandHeader,
                    PAGE_SIZE / size_of::<AHCICommandHeader>(),
                )
            };
            let cmd_table = unsafe { &mut *(cmd_table_va as *mut AHCICommandTable) };
            let identify_data = unsafe { &*(data_va as *mut ATAIdentifyPacket) };

            cmd_table.prdt[0].data_base_address = data_pa as u64;
            cmd_table.prdt[0].dbc_i = (BLOCK_SIZE - 1) as u32;

            cmd_list[0].command_table_base_address = cmd_table_pa as u64;
            cmd_list[0].prdt_length = 1;
            cmd_list[0].prd_byte_count = 0;

            port.command_list_base_address.write(cmd_list_pa as u64);
            port.fis_base_address.write(rfis_pa as u64);

            // clear status and errors
            port.command_issue.write(0);
            port.sata_active.write(0);
            port.sata_error.write(0);

            // enable port
            port.command.update(|c| {
                *c |= 1 << 0 | 1 << 1 | 1 << 2 | 1 << 4 | 1 << 28;
            });

            let stat = port.sata_status.read();
            if stat == 0 {
                warn!("port is not connected to external drive?");
            }

            let fis = &mut cmd_table.cfis;
            // Register FIS from HBA to device
            fis.fis_type = FIS_REG_H2D;
            fis.cflags = 1 << 7;

            // 7.15 IDENTIFY DEVICE - ECh, PIO Data-In
            fis.command = CMD_IDENTIFY_DEVICE;
            fis.sector_count = 1;

            debug!("issued identify command");
            port.issue_command(0);
            port.spin_on_slot(0);

            unsafe {
                debug!(
                    "Found ATA Device serial {} firmware {} model {} sectors 24bit={} 48bit={}",
                    from_ata_string(&identify_data.serial).trim_end(),
                    from_ata_string(&identify_data.firmware).trim_end(),
                    from_ata_string(&identify_data.model).trim_end(),
                    identify_data.lba_sectors,
                    identify_data.lba48_sectors,
                );
            }

            let data = unsafe { slice::from_raw_parts_mut(data_va as *mut u8, BLOCK_SIZE) };

            let driver = AHCIDriver(Mutex::new(AHCI {
                header,
                size,
                received_fis,
                cmd_list,
                cmd_table,
                data,
                port,
            }));

            let driver = Arc::new(driver);
            DRIVERS.write().push(driver.clone());
            BLK_DRIVERS
                .write()
                .push(Arc::new(BlockDriver(driver.clone())));

            return driver;
        }
    }

    unimplemented!();
}
