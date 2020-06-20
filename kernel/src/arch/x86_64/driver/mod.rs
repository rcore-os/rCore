pub mod keyboard;

use super::BootInfo;

pub fn init(_boot_info: &BootInfo) {
    keyboard::init();
}

pub fn init_graphic(boot_info: &BootInfo) {
    super::board::init_driver(boot_info);
    crate::drivers::console::init();
}
