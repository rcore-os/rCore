use super::bcm2837::timer::Timer;
use super::bcm2837::interrupt::{Controller, Interrupt};

pub fn handle_irq() {
    let controller = Timer::new();
    if controller.is_pending() {
        println!("Timer tick {}...", super::timer::get_cycle());
        super::timer::set_next();
        // ::trap::timer();
    }
}
