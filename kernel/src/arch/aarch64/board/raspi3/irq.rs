use crate::arch::interrupt::TrapFrame;
use bcm2837::timer::Timer;
use bcm2837::interrupt::{Controller, Interrupt};

pub fn handle_irq(tf: &mut TrapFrame) {
    let controller = Timer::new();
    if controller.is_pending() {
        super::timer::set_next();
        crate::trap::timer();
    }
}
