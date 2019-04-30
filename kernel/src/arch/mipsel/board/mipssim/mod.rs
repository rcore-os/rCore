use alloc::string::String;

#[path = "../../../../drivers/console/mod.rs"]
pub mod console;
pub mod consts;
#[path = "../../../../drivers/gpu/fb.rs"]
pub mod fb;
#[path = "../../../../drivers/serial/16550_reg.rs"]
pub mod serial;

/// Initialize serial port first
pub fn init_serial_early() {
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
