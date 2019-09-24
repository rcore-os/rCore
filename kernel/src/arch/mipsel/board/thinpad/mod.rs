use crate::drivers::gpu::fb::{self, FramebufferInfo};

pub mod consts;
#[path = "../../../../drivers/serial/simple_uart.rs"]
pub mod serial;

/// Device tree bytes
pub static DTB: &'static [u8] = include_bytes!("device.dtb");

/// Initialize serial port first
pub fn init_serial_early() {
    serial::init(0xa3000000);
    println!("Hello ThinPad!");
}

/// Initialize other board drivers
pub fn init_driver() {
    // TODO: add possibly more drivers
    // timer::init();

    let fb_info = FramebufferInfo {
        xres: 800,
        yres: 600,
        xres_virtual: 800,
        yres_virtual: 600,
        xoffset: 0,
        yoffset: 0,
        depth: fb::ColorDepth::ColorDepth8,
        format: fb::ColorFormat::RGB332,
        paddr: 0xa2000000,
        vaddr: 0xa2000000,
        screen_size: 800 * 600,
    };
    fb::init(fb_info);
}
