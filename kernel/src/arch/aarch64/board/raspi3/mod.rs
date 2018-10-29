//! Raspberry PI 3 Model B/B+

extern crate bcm2837;

pub mod serial;

pub fn init() {
    assert_has_not_been_called!("board::init must be called only once");

    serial::SERIAL_PORT.lock().init();

    println!("Hello Raspberry Pi!");
}
