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
const PCIM_CMD_PORTEN: u16 = 0x0001;
const PCIM_CMD_MEMEN: u16 = 0x0002;


fn pci_read_config(pci_base: usize, bus: u8, slot: u8, func: u8, offset: u8) -> u16 {
    // enable access mechanism
    let data = 0xF0 | (func << 1);
    write(pci_base + 0xcf8, data);
    write(pci_base + 0xcfa, bus);
    // calculate port address
    let addr: u16 = (0xC000 | ((slot as u16) << 8) | (offset as u16)) & 0xFFFC;
    // do the actual work
    read(pci_base + addr as usize)
}

fn pci_write_config(pci_base: usize, bus: u8, slot: u8, func: u8, offset: u8, value: u16) {
    // enable access mechanism
    let data = 0xF0 | (func << 1);
    write(pci_base + 0xcf8, data);
    write(pci_base + 0xcfa, bus);
    // calculate port address
    let addr: u16 = (0xC000 | ((slot as u16) << 8) | (offset as u16)) & 0xFFFC;
    // do the actual work
    write(pci_base + addr as usize, value);
}

pub fn init(pci_base: usize, vga_base: usize, x_res: u16, y_res: u16) {

    // enable PCI MMIO
    let pci_state = pci_read_config(pci_base, 0x00, 0x12, 0x00, PCIR_COMMAND);
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
    let vbe_enable = vga_read_vbe(VBE_DISPI_INDEX_ENABLE) | VBE_DISPI_ENABLED;
    vga_write_vbe(VBE_DISPI_INDEX_ENABLE, vbe_enable);

    println!("QEMU STDVGA driver initialized @ {:x}", vga_base);

}
