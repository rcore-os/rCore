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

pub fn init(vga_base: usize, x_res: u16, y_res: u16) {

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

}