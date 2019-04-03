use once::*;

pub mod serial;
pub mod fb;
#[path = "../../../../drivers/console/mod.rs"]
pub mod console;

/// Initialize serial port first
pub fn init_serial_early() {
    assert_has_not_been_called!("board::init must be called only once");
    serial::init();
    println!("Hello QEMU Malta!");
}

/// Initialize other board drivers
pub fn init_driver() {
    // TODO: add possibly more drivers
    // timer::init();
}