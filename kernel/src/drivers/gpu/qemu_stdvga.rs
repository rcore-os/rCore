//! driver for qemu stdvga (Cirrus)

use crate::util::{read, write};

const VGA_MMIO_OFFSET: usize = 0x400 - 0x3c0;
const VBE_MMIO_OFFSET: usize = 0x500;

const VGA_AR_ADDR: u16 = 0x3C0;
const VBE_DISPI_INDEX_XRES: u16 = 0x01;
const VBE_DISPI_INDEX_YRES: u16 = 0x02;
const VBE_DISPI_INDEX_BPP: u16 = 0x03;
const VBE_DISPI_INDEX_ENABLE: u16 = 0x04;

const VGA_AR_PAS: u8 =  0x20;
const VBE_DISPI_ENABLED: u16 = 0x01;

const PCIR_COMMAND: u8 = 0x04;
const PCIM_CMD_PORTEN: u32 = 0x0001;
const PCIM_CMD_MEMEN: u32 = 0x0002;


fn pci_read_config(pci_base: usize, bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    // write config address
    let address = (1 << 31) | ((bus as u32) << 16) | ((slot as u32) << 11) | ((func as u32) << 8) | (offset as u32);
    println!("Address: {:08x}", address);
    write(pci_base + 0xcf8, address);
    // do the actual work
    read(pci_base + 0xcfc)
}

fn pci_write_config(pci_base: usize, bus: u8, slot: u8, func: u8, offset: u8, value: u32) {
    // write config address
    let address = (1 << 31) | ((bus as u32) << 16) | ((slot as u32) << 11) | ((func as u32) << 8) | (offset as u32);
    write(pci_base + 0xcf8, address);
    // do the actual work
    write(pci_base + 0xcfc, value)
}

pub fn init(pci_base: usize, vga_base: usize, x_res: u16, y_res: u16) {

    // enable PCI MMIO
    let pci_vendor = pci_read_config(pci_base, 0x00, 0x12, 0x00, 0x0);
    println!("PCI Device ID: {:08x}", pci_vendor);

    let pci_state = pci_read_config(pci_base, 0x00, 0x12, 0x00, PCIR_COMMAND);
    println!("PCI Config Status: {:08x}", pci_state);
    pci_write_config(pci_base, 0x00, 0x12, 0x00, PCIR_COMMAND, pci_state | PCIM_CMD_PORTEN | PCIM_CMD_MEMEN);


    // vga operations
    let vga_write_io = |offset: u16, value: u8| {
        write(vga_base + VGA_MMIO_OFFSET + (offset as usize), value);
    };

    let vga_write_vbe = |offset: u16, value: u16| {
        write(vga_base + VBE_MMIO_OFFSET + (offset as usize) * 2, value);
    };

    let vga_read_vbe = |offset: u16| -> u16 {
        read(vga_base + VBE_MMIO_OFFSET + (offset as usize) * 2)
    };

    // enable palette access
    vga_write_io(VGA_AR_ADDR, VGA_AR_PAS);
    // set resolution and color depth
    vga_write_vbe(VBE_DISPI_INDEX_XRES, x_res);
    vga_write_vbe(VBE_DISPI_INDEX_YRES, y_res);
    vga_write_vbe(VBE_DISPI_INDEX_BPP, 8);
    // enable vbe
    let vbe_enable = vga_read_vbe(VBE_DISPI_INDEX_ENABLE);
    println!("VBE Status: {:04x}", vbe_enable);
    vga_write_vbe(VBE_DISPI_INDEX_ENABLE, vbe_enable | VBE_DISPI_ENABLED);

    println!("QEMU STDVGA driver initialized @ {:x}", vga_base);

}
