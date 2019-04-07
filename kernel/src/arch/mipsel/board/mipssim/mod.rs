use once::*;
use alloc::string::String;

#[path = "../../../../drivers/serial/16550_reg.rs"]
pub mod serial;
#[path = "../../../../drivers/gpu/fb.rs"]
pub mod fb;
#[path = "../../../../drivers/console/mod.rs"]
pub mod console;
pub mod consts;

/// Initialize serial port first
pub fn init_serial_early() {
    assert_has_not_been_called!("board::init must be called only once");
    serial::init(0xbfd003f8);
    println!("Hello QEMU MIPSSIM!");
}

/// Initialize other board drivers
pub fn init_driver() {
    // TODO: add possibly more drivers
    // timer::init();
}

pub fn probe_fb_info(_width: u32, _height: u32, _depth: u32) -> fb::FramebufferResult {
    Err(String::from("Framebuffer not usable on mipssim board"))
}