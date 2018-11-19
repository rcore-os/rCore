use arch::interrupt::TrapFrame;
use super::bcm2837::timer::Timer;
use super::bcm2837::interrupt::{Controller, Interrupt};

pub fn handle_irq(tf: &mut TrapFrame) {
    let controller = Timer::new();
    if controller.is_pending() {
        super::timer::set_next();
        ::trap::timer();
    }
}
