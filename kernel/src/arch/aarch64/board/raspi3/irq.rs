use crate::arch::interrupt::TrapFrame;
use bcm2837::interrupt::Controller;
use log::*;

pub use bcm2837::interrupt::Interrupt;

static IRQ_HANDLERS: &'static [Option<fn()>; 64] = &[None; 64];

pub fn handle_irq(tf: &mut TrapFrame) {
    let controller = bcm2837::timer::Timer::new();
    if controller.is_pending() {
        super::timer::set_next();
        crate::trap::timer();
    }

    for int in Controller::new().pending_interrupts() {
        if let Some(handler) = IRQ_HANDLERS[int] {
            handler();
        } else {
            error!("Unregistered IRQ {}", int);
            crate::trap::error(tf);
        }
    }
}

pub fn register_irq(int: Interrupt, handler: fn()) {
    unsafe {
        *(&IRQ_HANDLERS[int as usize] as *const _ as *mut Option<fn()>) = Some(handler);
    }
    Controller::new().enable(int);
}
