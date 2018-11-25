//! Raspberry PI 3 Model B/B+

extern crate bcm2837;

pub mod irq;
pub mod timer;
pub mod serial;

pub const IO_REMAP_BASE: usize = bcm2837::IO_BASE;
pub const IO_REMAP_END: usize = 0x40001000;

pub fn init() {
    assert_has_not_been_called!("board::init must be called only once");

    serial::SERIAL_PORT.lock().init();

    println!("Hello Raspberry Pi!");
}
