pub mod ide;
pub mod keyboard;
pub mod rtc_cmos;
pub mod serial;

use super::BootInfo;

pub fn init(boot_info: &BootInfo) {
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

pub fn init_graphic(boot_info: &BootInfo) {
    super::board::init_driver(boot_info);
    crate::drivers::console::init();
}
