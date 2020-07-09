use crate::drivers::block::ide;
use crate::drivers::bus::pci;
use crate::drivers::gpu::fb::{self, FramebufferInfo};
use crate::drivers::*;
use mips::registers::cp0;

pub mod consts;
#[path = "../../../../drivers/gpu/qemu_stdvga.rs"]
pub mod vga;

/// Device tree bytes
pub static DTB: &'static [u8] = include_bytes!("device.dtb");

/// Initialize serial port first
pub fn early_init() {
    // Enable serial interrupt
    let mut status = cp0::status::read();
    status.enable_hard_int2();
    cp0::status::write(status);
    info!("Hello QEMU Malta!");
}

/// Initialize other board drivers
pub fn init(dtb: usize) {
    // TODO: add possibly more drivers
    serial::uart16550::driver_init();
    vga::init(0xbbe00000, 0xb2050000, 800, 600);
    pci::init();

    let fb_info = FramebufferInfo {
        xres: 800,
        yres: 600,
        xres_virtual: 800,
        yres_virtual: 600,
        xoffset: 0,
        yoffset: 0,
        depth: fb::ColorDepth::ColorDepth8,
        format: fb::ColorFormat::VgaPalette,
        paddr: 0xb0000000,
        vaddr: 0xb0000000,
        screen_size: 800 * 600,
    };
    fb::init(fb_info);
    ide::init();
    device_tree::init(dtb);
}
