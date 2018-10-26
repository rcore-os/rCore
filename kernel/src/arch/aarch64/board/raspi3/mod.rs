//! Raspberry PI 3 Model B/B+

extern crate bcm2837;

pub fn init() {
    // TODO
    bcm2837::gpio::Gpio::new(14).set_gpio_pd(0);
    bcm2837::gpio::Gpio::new(15).set_gpio_pd(0);
}
