//! Raspberry PI 3 Model B/B+

extern crate bcm2837;

pub mod serial;

pub fn init() {
    // FIXME
    // assert_has_not_been_called!("board::init must be called only once");

    unsafe {
        serial::SERIAL_PORT.init();
    }

    println!("Hello Raspberry Pi!");
}
