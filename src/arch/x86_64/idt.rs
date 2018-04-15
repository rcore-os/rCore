use x86_64::structures::idt::Idt;
use spin::Once;

static IDT: Once<Idt> = Once::new();

pub fn init() {
    let idt = IDT.call_once(|| {
        use arch::interrupt::irq::*;
        use consts::irq::*;
		use arch::gdt::DOUBLE_FAULT_IST_INDEX;
        
        let mut idt = Idt::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);
        idt[(T_IRQ0 + IRQ_COM1) as usize].set_handler_fn(serial_handler);
        idt[(T_IRQ0 + IRQ_KBD) as usize].set_handler_fn(keyboard_handler);
        idt[(T_IRQ0 + IRQ_TIMER) as usize].set_handler_fn(timer_handler);
        unsafe {
            idt.page_fault.set_handler_fn(page_fault_handler)
                .set_stack_index(DOUBLE_FAULT_IST_INDEX as u16);
        }
        idt
    });

    idt.load();
}