use once::*;
use alloc::string::String;

pub mod serial;
#[path = "../../../../drivers/gpu/fb.rs"]
pub mod fb;
#[path = "../../../../drivers/console/mod.rs"]
pub mod console;

/// Initialize serial port first
pub fn init_serial_early() {
    assert_has_not_been_called!("board::init must be called only once");
    serial::init(0xbf000900);
    println!("Hello QEMU Malta!");
}

/// Initialize other board drivers
pub fn init_driver() {
    // TODO: add possibly more drivers
    // timer::init();
}

pub fn probe_fb_info(width: u32, height: u32, depth: u32) -> Result<(fb::FramebufferInfo, usize), String> {
    Err(String::from("Framebuffer not usable on malta board"))
}