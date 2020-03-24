use crate::arch::interrupt::TrapFrame;
use bcm2837::interrupt::Controller;
use spin::RwLock;

pub use bcm2837::interrupt::Interrupt;

lazy_static! {
    static ref IRQ_HANDLERS: RwLock<[Option<fn()>; 64]> = RwLock::new([None; 64]);
}

pub fn is_timer_irq() -> bool {
    super::timer::is_pending()
}

pub fn handle_irq(_tf: &mut TrapFrame) {
    for int in Controller::new().pending_interrupts() {
        if let Some(handler) = IRQ_HANDLERS.read()[int] {
            handler();
        }
    }
}

pub fn register_irq(int: Interrupt, handler: fn()) {
    IRQ_HANDLERS.write()[int as usize] = Some(handler);
    Controller::new().enable(int);
}
