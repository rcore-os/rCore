use once::*;
use alloc::string::String;

#[path = "../../../../drivers/serial/simple_uart.rs"]
pub mod serial;
#[path = "../../../../drivers/gpu/fb.rs"]
pub mod fb;
#[path = "../../../../drivers/console/mod.rs"]
pub mod console;
pub mod consts;

use fb::FramebufferInfo;
use fb::FramebufferResult;

/// Initialize serial port first
pub fn init_serial_early() {
    assert_has_not_been_called!("board::init must be called only once");
    serial::init(0xa3000000);
    println!("Hello ThinPad!");
}

/// Initialize other board drivers
pub fn init_driver() {
    // TODO: add possibly more drivers
    // timer::init();
}

pub fn probe_fb_info(width: u32, height: u32, depth: u32) -> FramebufferResult {
    let fb_info = FramebufferInfo {
        xres: 800,
        yres: 600,
        xres_virtual: 800,
        yres_virtual: 600,
        xoffset: 0,
        yoffset: 0,
        depth: 8,
        pitch: 800,
        bus_addr: 0xa2000000,
        screen_size: 800 * 600,
    };
    Ok((fb_info, fb::ColorConfig::RGB332, 0xa2000000))
}