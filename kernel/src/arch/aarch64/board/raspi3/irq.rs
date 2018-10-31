use super::bcm2837::interrupt::{Controller, Interrupt};

pub fn handle_irq() {
    let controller = Controller::new();
    if controller.is_pending(Interrupt::Timer1) {
        println!("Timer tick...");
        super::timer::set_next();
        // ::trap::timer();
    }
}
