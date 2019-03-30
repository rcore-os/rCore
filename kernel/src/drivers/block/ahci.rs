//! Driver for AHCI
//!
//! Spec: https://www.intel.com/content/dam/www/public/us/en/documents/technical-specifications/serial-ata-ahci-spec-rev1-3-1.pdf

use alloc::alloc::{alloc_zeroed, Layout};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cmp::min;
use core::mem::size_of;
use core::slice;

use crate::sync::FlagsGuard;
use bitflags::*;
use log::*;
use rcore_memory::paging::PageTable;
use rcore_memory::PAGE_SIZE;
use volatile::Volatile;

use rcore_fs::dev::BlockDevice;

use crate::drivers::BlockDriver;
use crate::memory::active_table;
use crate::sync::SpinNoIrqLock as Mutex;

use super::super::{DeviceType, Driver, BLK_DRIVERS, DRIVERS};

pub struct AHCI {
    header: usize,
    size: usize,
    rfis: usize,
    cmd_list: usize,
    cmd_table: usize,
    data: usize,
    port_addr: usize,
}

pub struct AHCIDriver(Mutex<AHCI>);

// AHCI
// 3.1 Generic Host Control
#[repr(C)]
pub struct AHCIGHC {
    cap: Volatile<AHCICap>,
    ghc: Volatile<u32>,
    is: Volatile<u32>,
    pi: Volatile<u32>,
    vs: Volatile<u32>,
    ccc_ctl: Volatile<u32>,
    ccc_ports: Volatile<u32>,
    em_loc: Volatile<u32>,
    em_ctl: Volatile<u32>,
    cap2: Volatile<u32>,
    bohc: Volatile<u32>,
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
        const NP_1 = 1 << 0;
        const NP_2 = 1 << 1;
        const NP_4 = 1 << 2;
        const NP_8 = 1 << 3;
        const NP_16 = 1 << 4;
    }
}

// AHCI
// 3.3 Port Registers (one set per port)
#[repr(C)]
pub struct AHCIPort {
    clb: Volatile<u32>,
    clbu: Volatile<u32>,
    fb: Volatile<u32>,
    fbu: Volatile<u32>,
    is: Volatile<u32>,
    ie: Volatile<u32>,
    cmd: Volatile<u32>,
    reserved: Volatile<u32>,
    tfd: Volatile<u32>,
    sig: Volatile<u32>,
    ssts: Volatile<u32>,
    sctl: Volatile<u32>,
    serr: Volatile<u32>,
    sact: Volatile<u32>,
    ci: Volatile<u32>,
    sntf: Volatile<u32>,
    fbs: Volatile<u32>,
    devslp: Volatile<u32>,
}

// AHCi
// 4.2.1 Received FIS Structure
#[repr(C)]
pub struct AHCIRFIS {
    dma: [u8; 0x20],
    pio: [u8; 0x20],
    d2h: [u8; 0x18],
    sdbfis: [u8; 0x8],
    ufis: [u8; 0x40],
    reserved: [u8; 0x60],
}

// AHCI
// 4.2.2 Command List Structure
#[repr(C)]
pub struct AHCICommandHeader {
    pwa_cfl: u8,
    pmp_cbr: u8,
    prdtl: u16,
    prdbc: u32,
    ctba0: u32,
    ctba_u0: u32,
    reservec: [u32; 4],
}

// AHCI
// 4.2.3 Command Table
#[repr(C)]
pub struct AHCICommandTable {
    cfis: [u8; 64],
    acmd: [u8; 16],
    reserved: [u8; 48],
    prdt: [AHCIPRD; 1],
}

// 4.2.3 Command Table
#[repr(C)]
pub struct AHCIPRD {
    dba: u32,
    dbau: u32,
    reserved: u32,
    dbc_i: u32,
}

const FIS_REG_H2D: u8 = 0x27;

const CMD_READ_DMA_EXT: u8 = 0x25;
const CMD_WRITE_DMA_EXT: u8 = 0x35;
const CMD_IDENTIFY_DEVICE: u8 = 0xec;

// https://wiki.osdev.org/AHCI
// SATA
// Figure 5-2: Register FIS - Host to Device
#[repr(C)]
#[derive(Default)]
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
    sector_count_lo: u8,
    sector_count_hi: u8,
    reserved: u8,
    control: u8,
}

// ATA8-ACS
// Table 29 IDENTIFY DEVICE data
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

impl Driver for AHCIDriver {
    fn try_handle_interrupt(&self, _irq: Option<u32>) -> bool {
        let driver = self.0.lock();

        // ensure header page is mapped
        let header = driver.header as usize;
        let size = driver.size as usize;
        if let None = active_table().get_entry(header) {
            let mut current_addr = header;
            while current_addr < header + size {
                active_table().map_if_not_exists(current_addr, current_addr);
                current_addr = current_addr + PAGE_SIZE;
            }
        }
        return false;
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Block
    }

    fn get_id(&self) -> String {
        format!("ahci")
    }

    fn read_block(&self, block_id: usize, buf: &mut [u8]) -> bool {
        let mut driver = self.0.lock();
        // ensure header page is mapped
        let header = driver.header as usize;
        let size = driver.size as usize;
        if let None = active_table().get_entry(header) {
            let mut current_addr = header;
            while current_addr < header + size {
                active_table().map_if_not_exists(current_addr, current_addr);
                current_addr = current_addr + PAGE_SIZE;
            }
        }
        let port = unsafe { &mut *(driver.port_addr as *mut AHCIPort) };
        let cmd_headers = unsafe {
            slice::from_raw_parts_mut(
                driver.cmd_list as *mut AHCICommandHeader,
                PAGE_SIZE / size_of::<AHCICommandHeader>(),
            )
        };
        cmd_headers[0].prdbc = 0;
        cmd_headers[0].pwa_cfl = 0;

        let cmd_table = unsafe { &mut *(driver.cmd_table as *mut AHCICommandTable) };
        let len = min(BLOCK_SIZE, buf.len());
        let data = unsafe { slice::from_raw_parts(driver.data as *const u8, len) };

        let fis = unsafe { &mut *(cmd_table.cfis.as_ptr() as *mut SATAFISRegH2D) };
        // Register FIS from HBA to device
        fis.fis_type = FIS_REG_H2D;
        fis.cflags = 1 << 7;

        // 7.25 READ DMA EXT - 25h, DMA
        fis.command = CMD_READ_DMA_EXT;
        fis.sector_count_lo = 1;
        fis.sector_count_hi = 0;
        fis.dev_head = 0x40; // LBA
        fis.control = 0x80; // LBA48

        let block_id = block_id as u64; // avoid problems on riscv32
        fis.lba_0 = block_id as u8;
        fis.lba_1 = (block_id >> 8) as u8;
        fis.lba_2 = (block_id >> 16) as u8;
        fis.lba_3 = (block_id >> 24) as u8;
        fis.lba_4 = (block_id >> 32) as u8;
        fis.lba_5 = (block_id >> 40) as u8;

        port.ci.write(1 << 0);

        loop {
            let ci = port.ci.read();
            if (ci & (1 << 0)) == 0 {
                break;
            }
        }

        (&mut buf[..len]).clone_from_slice(&data);
        return true;
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) -> bool {
        if buf.len() < BLOCK_SIZE {
            return false;
        }
        let mut driver = self.0.lock();
        // ensure header page is mapped
        let header = driver.header as usize;
        let size = driver.size as usize;
        if let None = active_table().get_entry(header) {
            let mut current_addr = header;
            while current_addr < header + size {
                active_table().map_if_not_exists(current_addr, current_addr);
                current_addr = current_addr + PAGE_SIZE;
            }
        }
        let port = unsafe { &mut *(driver.port_addr as *mut AHCIPort) };
        let cmd_headers = unsafe {
            slice::from_raw_parts_mut(
                driver.cmd_list as *mut AHCICommandHeader,
                PAGE_SIZE / size_of::<AHCICommandHeader>(),
            )
        };
        cmd_headers[0].prdbc = 0;
        cmd_headers[0].pwa_cfl = 1 << 6; // devic write

        let cmd_table = unsafe { &mut *(driver.cmd_table as *mut AHCICommandTable) };
        let data = unsafe { slice::from_raw_parts_mut(driver.data as *mut u8, BLOCK_SIZE) };
        data.clone_from_slice(&buf[..BLOCK_SIZE]);

        let fis = unsafe { &mut *(cmd_table.cfis.as_ptr() as *mut SATAFISRegH2D) };
        // Register FIS from HBA to device
        fis.fis_type = FIS_REG_H2D;
        fis.cflags = 1 << 7;

        // ATA8-ACS
        // 7.63 WRITE DMA EXT - 35h, DMA
        fis.command = CMD_WRITE_DMA_EXT;
        fis.sector_count_lo = 1;
        fis.sector_count_hi = 0;
        fis.dev_head = 0x40; // LBA
        fis.control = 0x80; // LBA48

        let block_id = block_id as u64; // avoid problems on riscv32
        fis.lba_0 = block_id as u8;
        fis.lba_1 = (block_id >> 8) as u8;
        fis.lba_2 = (block_id >> 16) as u8;
        fis.lba_3 = (block_id >> 24) as u8;
        fis.lba_4 = (block_id >> 32) as u8;
        fis.lba_5 = (block_id >> 40) as u8;

        port.ci.write(1 << 0);

        loop {
            let ci = port.ci.read();
            if (ci & (1 << 0)) == 0 {
                break;
            }
        }
        return true;
    }
}

const BLOCK_SIZE: usize = 512;

fn from_ata_string(data: &[u8]) -> String {
    let mut swapped_data = Vec::new();
    assert!(data.len() % 2 == 0);
    for i in (0..data.len()).step_by(2) {
        swapped_data.push(data[i + 1]);
        swapped_data.push(data[i]);
    }
    return String::from_utf8(swapped_data).unwrap();
}

pub fn ahci_init(irq: Option<u32>, header: usize, size: usize) -> Arc<AHCIDriver> {
    let _ = FlagsGuard::no_irq_region();
    let mut current_addr = header;
    while current_addr < header + size {
        active_table().map_if_not_exists(current_addr, current_addr);
        current_addr = current_addr + PAGE_SIZE;
    }
    let ghc = unsafe { &mut *(header as *mut AHCIGHC) };

    // AHCI Enable
    ghc.ghc.write(ghc.ghc.read() | (1 << 13));

    let num_ports = (ghc.cap.read().bits() & 0x1f) as usize + 1;

    for port_num in 0..num_ports {
        if (ghc.pi.read() | (1 << port_num)) != 0 {
            let addr = header + 0x100 + 0x80 * port_num;
            let port = unsafe { &mut *(addr as *mut AHCIPort) };

            // SSTS IPM Active
            if (port.ssts.read() >> 8) & 0xF != 1 {
                continue;
            }

            // SSTS DET Present
            if port.ssts.read() & 0xF != 3 {
                continue;
            }

            debug!("probing port {}", port_num);
            // Disable Port First
            port.cmd.write(port.cmd.read() & !(1 << 4 | 1 << 0));

            let rfis_va =
                unsafe { alloc_zeroed(Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap()) }
                    as usize;
            let rfis_pa = active_table().get_entry(rfis_va).unwrap().target();

            let cmd_list_va =
                unsafe { alloc_zeroed(Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap()) }
                    as usize;
            let cmd_list_pa = active_table().get_entry(cmd_list_va).unwrap().target();

            let cmd_table_va =
                unsafe { alloc_zeroed(Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap()) }
                    as usize;
            let cmd_table_pa = active_table().get_entry(cmd_table_va).unwrap().target();

            let data_va =
                unsafe { alloc_zeroed(Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap()) }
                    as usize;
            let data_pa = active_table().get_entry(data_va).unwrap().target();

            let cmd_headers = unsafe {
                slice::from_raw_parts_mut(
                    cmd_list_va as *mut AHCICommandHeader,
                    PAGE_SIZE / size_of::<AHCICommandHeader>(),
                )
            };

            let cmd_table = unsafe { &mut *(cmd_table_va as *mut AHCICommandTable) };

            cmd_table.prdt[0].dba = data_pa as u32;
            cmd_table.prdt[0].dbau = (data_pa >> 32) as u32;
            cmd_table.prdt[0].dbc_i = (BLOCK_SIZE - 1) as u32;

            cmd_headers[0].ctba0 = cmd_table_pa as u32;
            cmd_headers[0].ctba_u0 = (cmd_table_pa >> 32) as u32;
            cmd_headers[0].prdtl = 1;
            cmd_headers[0].prdbc = 0;

            port.clb.write(cmd_list_pa as u32);
            port.clbu.write((cmd_list_pa >> 32) as u32);

            port.fb.write(rfis_pa as u32);
            port.fbu.write((rfis_pa >> 32) as u32);

            // clear status and errors
            port.ci.write(0);
            port.sact.write(0);
            port.serr.write(0);

            // enable port
            port.cmd
                .write(port.cmd.read() | 1 << 0 | 1 << 1 | 1 << 2 | 1 << 4 | 1 << 28);

            let stat = port.ssts.read();
            if stat == 0 {
                warn!("port is not connected to external drive?");
            }

            let fis = unsafe { &mut *(cmd_table.cfis.as_ptr() as *mut SATAFISRegH2D) };
            // Register FIS from HBA to device
            fis.fis_type = FIS_REG_H2D;
            fis.cflags = 1 << 7;

            // 7.15 IDENTIFY DEVICE - ECh, PIO Data-In
            fis.command = CMD_IDENTIFY_DEVICE;
            fis.sector_count_lo = 1;

            debug!("issued identify command");
            port.ci.write(1 << 0);

            loop {
                let ci = port.ci.read();
                if (ci & (1 << 0)) == 0 {
                    break;
                }
            }

            let identify_data = unsafe { &*(data_va as *mut ATAIdentifyPacket) };

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

            let driver = AHCIDriver(Mutex::new(AHCI {
                header,
                size,
                rfis: rfis_va,
                cmd_list: cmd_list_va,
                cmd_table: cmd_table_va,
                data: data_va,
                port_addr: addr,
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
