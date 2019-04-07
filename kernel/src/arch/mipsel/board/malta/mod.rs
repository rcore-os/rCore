use once::*;
use alloc::string::String;
use mips::registers::cp0;

#[path = "../../../../drivers/serial/ti_16c550c.rs"]
pub mod serial;
#[path = "../../../../drivers/gpu/fb.rs"]
pub mod fb;
#[path = "../../../../drivers/console/mod.rs"]
pub mod console;
pub mod consts;

/// Initialize serial port first
pub fn init_serial_early() {
    assert_has_not_been_called!("board::init must be called only once");
    // initialize serial driver
    serial::init(0xbf000900);
    // Enable serial interrupt
    unsafe {
        let mut status = cp0::status::read();
        status.enable_hard_int2();
        cp0::status::write(status);
    }
    println!("Hello QEMU Malta!");
}

/// Initialize other board drivers
pub fn init_driver() {
    // TODO: add possibly more drivers
    // timer::init();
}

pub fn probe_fb_info(_width: u32, _height: u32, _depth: u32) -> fb::FramebufferResult {
    Err(String::from("Framebuffer not usable on malta board"))
}