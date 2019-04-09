//! driver for qemu stdvga (Cirrus)

use crate::util::{read, write};

const VGA_MMIO_OFFSET: usize = 0x400 - 0x3C0;
const VBE_MMIO_OFFSET: usize = 0x500;

const VGA_AR_ADDR: u16 = 0x3C0;
const VBE_DISPI_INDEX_XRES: u16 = 0x1;
const VBE_DISPI_INDEX_YRES: u16 = 0x2;
const VBE_DISPI_INDEX_BPP: u16 = 0x3;
const VBE_DISPI_INDEX_ENABLE: u16 = 0x4;
const VBE_DISPI_INDEX_BANK: u16 = 0x5;
const VBE_DISPI_INDEX_VIRT_WIDTH: u16 = 0x6;
const VBE_DISPI_INDEX_VIRT_HEIGHT: u16 = 0x7;
const VBE_DISPI_INDEX_X_OFFSET: u16 = 0x8;
const VBE_DISPI_INDEX_Y_OFFSET: u16 = 0x9;
const VBE_DISPI_INDEX_VIDEO_MEMORY_64K: u16 = 0xa;

const VGA_AR_PAS: u8 = 0x20;
const VBE_DISPI_ENABLED: u16 = 0x01;
const VBE_DISPI_8BIT_DAC: u16 = 0x20;
const VBE_DISPI_LFB_ENABLED: u16 = 0x40;

const PCI_COMMAND: u8 = 0x04;
const PCI_COMMAND_IO: u32 = 0x1;
const PCI_COMMAND_MEMORY: u32 = 0x2;
const PCI_COMMAND_MASTER: u32 = 0x4;
const PCI_COMMAND_SPECIAL: u32 = 0x8;
const PCI_COMMAND_SERR: u32 = 0x100;

fn pci_read_config(pci_base: usize, bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    // write config address
    let address = (1 << 31)
        | ((bus as u32) << 16)
        | ((slot as u32) << 11)
        | ((func as u32) << 8)
        | (offset as u32);
    write(pci_base + 0xcf8, address);
    // do the actual work
    let value = read(pci_base + 0xcfc);
    debug!(
        "Read {:08x} from PCI address: {:02x}:{:02x}.{:02x} @ 0x{:02x}",
        value, bus, slot, func, offset
    );
    value
}

fn pci_write_config(pci_base: usize, bus: u8, slot: u8, func: u8, offset: u8, value: u32) {
    // write config address
    let address = (1 << 31)
        | ((bus as u32) << 16)
        | ((slot as u32) << 11)
        | ((func as u32) << 8)
        | (offset as u32);
    debug!(
        "Write {:08x} to PCI address: {:02x}:{:02x}.{:02x} @ 0x{:02x}",
        value, bus, slot, func, offset
    );
    write(pci_base + 0xcf8, address);
    // do the actual work
    write(pci_base + 0xcfc, value)
}

pub fn init(pci_base: usize, vga_base: usize, x_res: u16, y_res: u16) {
    debug!(
        "PCI Controller Base: {:08x}",
        pci_read_config(pci_base, 0x00, 0x00, 0x00, 0x20)
    );

    let controller = pci_read_config(pci_base, 0x00, 0x00, 0x00, PCI_COMMAND);
    pci_write_config(
        pci_base,
        0x00,
        0x00,
        0x00,
        PCI_COMMAND,
        controller | PCI_COMMAND_MASTER | PCI_COMMAND_IO | PCI_COMMAND_MEMORY | PCI_COMMAND_SERR,
    );

    let pci_vendor = pci_read_config(pci_base, 0x00, 0x12, 0x00, 0x0);
    debug!("VGA PCI Device ID: {:08x}", pci_vendor);

    // enable port and MMIO for vga card
    pci_write_config(
        pci_base,
        0x00,
        0x12,
        0x00,
        PCI_COMMAND,
        pci_read_config(pci_base, 0x00, 0x12, 0x00, PCI_COMMAND) | PCI_COMMAND_MEMORY,
    );
    // bar 0
    pci_write_config(pci_base, 0x00, 0x12, 0x00, 0x10, 0x10000000);
    debug!(
        "VGA PCI BAR 0: {:08x}",
        pci_read_config(pci_base, 0x00, 0x12, 0x00, 0x10)
    );
    // bar 2
    pci_write_config(pci_base, 0x00, 0x12, 0x00, 0x18, 0x12050000);
    debug!(
        "VGA PCI BAR 2: {:08x}",
        pci_read_config(pci_base, 0x00, 0x12, 0x00, 0x18)
    );

    // vga operations
    let vga_write_io = |offset: u16, value: u8| {
        write(vga_base + VGA_MMIO_OFFSET + (offset as usize), value);
    };

    let vga_read_io = |offset: u16| -> u8 { read(vga_base + VGA_MMIO_OFFSET + (offset as usize)) };

    let vga_write_vbe = |offset: u16, value: u16| {
        write(vga_base + VBE_MMIO_OFFSET + (offset as usize) * 2, value);
    };

    let vga_read_vbe =
        |offset: u16| -> u16 { read(vga_base + VBE_MMIO_OFFSET + (offset as usize) * 2) };

    debug!("VGA Endianess: {:x}", read::<u32>(vga_base + 0x604));

    // unblank vga output
    vga_write_io(VGA_AR_ADDR, VGA_AR_PAS);
    debug!("VGA AR: {}", vga_read_io(VGA_AR_ADDR));

    vga_write_vbe(VBE_DISPI_INDEX_ENABLE, 0);

    // set resolution and color depth
    vga_write_vbe(VBE_DISPI_INDEX_XRES, x_res);
    vga_write_vbe(VBE_DISPI_INDEX_YRES, y_res);
    vga_write_vbe(VBE_DISPI_INDEX_VIRT_WIDTH, x_res);
    vga_write_vbe(VBE_DISPI_INDEX_VIRT_HEIGHT, y_res);
    vga_write_vbe(VBE_DISPI_INDEX_BANK, 0);
    vga_write_vbe(VBE_DISPI_INDEX_X_OFFSET, 0);
    vga_write_vbe(VBE_DISPI_INDEX_Y_OFFSET, 0);
    vga_write_vbe(VBE_DISPI_INDEX_BPP, 8);
    debug!(
        "VGA Resolution: {}*{}@{}bit",
        vga_read_vbe(VBE_DISPI_INDEX_XRES),
        vga_read_vbe(VBE_DISPI_INDEX_YRES),
        vga_read_vbe(VBE_DISPI_INDEX_BPP)
    );

    // enable vbe
    let vbe_enable = vga_read_vbe(VBE_DISPI_INDEX_ENABLE);
    vga_write_vbe(
        VBE_DISPI_INDEX_ENABLE,
        vbe_enable | VBE_DISPI_ENABLED | VBE_DISPI_LFB_ENABLED | VBE_DISPI_8BIT_DAC,
    );
    debug!("VBE Status: {:04x}", vga_read_vbe(VBE_DISPI_INDEX_ENABLE));

    info!("QEMU STDVGA driver initialized @ {:x}", vga_base);
}
