extern crate syscall as redox_syscall;

pub mod vga;
pub mod apic;
pub mod serial;
pub mod pic;
pub mod keyboard;
pub mod pit;
pub mod ide;

pub fn init() {
    assert_has_not_been_called!();

    if cfg!(feature = "use_apic") {
        pic::disable();
        apic::init();
    } else {
        pic::init();
    }
    pit::init();
    serial::init();
    keyboard::init();
}