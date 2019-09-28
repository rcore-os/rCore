use crate::arch::interrupt::TrapFrame;
use bcm2837::interrupt::Controller;

pub use bcm2837::interrupt::Interrupt;

static IRQ_HANDLERS: &'static [Option<fn()>; 64] = &[None; 64];

pub fn is_timer_irq() -> bool {
    super::timer::is_pending()
}

pub fn handle_irq(_tf: &mut TrapFrame) {
    for int in Controller::new().pending_interrupts() {
        if let Some(handler) = IRQ_HANDLERS[int] {
            handler();
        }
    }
}

pub fn register_irq(int: Interrupt, handler: fn()) {
    unsafe {
        *(&IRQ_HANDLERS[int as usize] as *const _ as *mut Option<fn()>) = Some(handler);
    }
    Controller::new().enable(int);
}
