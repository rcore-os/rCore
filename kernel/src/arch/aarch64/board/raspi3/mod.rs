//! Raspberry PI 3 Model B/B+

pub mod irq;
pub mod timer;
pub mod serial;

pub fn init() {
    // FIXME
    // assert_has_not_been_called!("board::init must be called only once");

    unsafe {
        serial::SERIAL_PORT.init();
    }

    println!("Hello Raspberry Pi!");
}
