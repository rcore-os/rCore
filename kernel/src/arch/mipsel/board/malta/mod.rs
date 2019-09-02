use crate::drivers::bus::pci;
use alloc::string::String;
use mips::registers::cp0;

#[path = "../../../../drivers/console/mod.rs"]
pub mod console;
pub mod consts;
#[path = "../../../../drivers/gpu/fb.rs"]
pub mod fb;
#[path = "../../../../drivers/serial/ti_16c550c.rs"]
pub mod serial;
#[path = "../../../../drivers/gpu/qemu_stdvga.rs"]
pub mod vga;

use fb::FramebufferInfo;

/// Device tree bytes
pub static DTB: &'static [u8] = include_bytes!("device.dtb");

/// Initialize serial port first
pub fn init_serial_early() {
    // initialize serial driver
    serial::init(0xbf000900);
    // Enable serial interrupt
    let mut status = cp0::status::read();
    status.enable_hard_int2();
    cp0::status::write(status);
    println!("Hello QEMU Malta!");
}

/// Initialize other board drivers
pub fn init_driver() {
    // TODO: add possibly more drivers
    vga::init(0xbbe00000, 0xb2050000, 800, 600);
    pci::init();
    fb::init();
}

pub fn probe_fb_info(_width: u32, _height: u32, _depth: u32) -> fb::FramebufferResult {
    Ok(FramebufferInfo {
        xres: 800,
        yres: 600,
        xres_virtual: 800,
        yres_virtual: 600,
        xoffset: 0,
        yoffset: 0,
        depth: fb::ColorDepth::try_from(8)?,
        format: fb::ColorFormat::VgaPalette,
        paddr: 0xb0000000,
        vaddr: 0xb0000000,
        screen_size: 800 * 600,
    })
}
