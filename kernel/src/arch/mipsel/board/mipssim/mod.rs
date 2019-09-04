pub mod consts;
#[path = "../../../../drivers/serial/16550_reg.rs"]
pub mod serial;

/// Device tree bytes
pub static DTB: &'static [u8] = include_bytes!("device.dtb");

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
