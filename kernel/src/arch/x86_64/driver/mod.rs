use once::*;
use crate::arch::interrupt::{consts, enable_irq};

pub mod vga;
pub mod serial;
pub mod pic;
pub mod keyboard;
pub mod pit;
pub mod ide;
pub mod rtc_cmos;

pub fn init() {
    assert_has_not_been_called!();

    // Use IOAPIC instead of PIC
    pic::disable();

    // Use APIC Timer instead of PIT
    // pit::init();

    serial::init();
    keyboard::init();

    // Enable PCI Interrupts when necessary
    // because they can be shared among devices
    // including mouse and keyboard
    /*
    enable_irq(consts::PIRQA);
    enable_irq(consts::PIRQB);
    enable_irq(consts::PIRQC);
    enable_irq(consts::PIRQD);
    enable_irq(consts::PIRQE);
    enable_irq(consts::PIRQF);
    enable_irq(consts::PIRQG);
    enable_irq(consts::PIRQH);
    */
}