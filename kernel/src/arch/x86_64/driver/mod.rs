extern crate syscall as redox_syscall;

pub mod vga;
pub mod serial;
pub mod pic;
pub mod keyboard;
pub mod pit;
pub mod ide;

pub fn init() {
    assert_has_not_been_called!();

    // Use IOAPIC instead of PIC
    pic::disable();

    // Use APIC Timer instead of PIT
    // pit::init();

    serial::init();
    keyboard::init();
}