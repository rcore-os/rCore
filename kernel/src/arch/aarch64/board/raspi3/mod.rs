//! Raspberry PI 3 Model B/B+

use once::*;

pub mod irq;
pub mod timer;
pub mod serial;
pub mod mailbox;

pub const IO_REMAP_BASE: usize = bcm2837::IO_BASE;
pub const IO_REMAP_END: usize = 0x40001000;

/// Some initializations must be done before other initializations.
pub fn init_early() {
    assert_has_not_been_called!("board::init must be called only once");

    serial::SERIAL_PORT.lock().init();

    println!("Hello Raspberry Pi!");
}

/// Initialize raspi3 drivers
pub fn init_driver() {
    timer::init();
}
