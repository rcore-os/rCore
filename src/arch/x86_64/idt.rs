use x86_64::structures::idt::Idt;
use spin::Once;
use alloc::boxed::Box;

/// Alloc IDT at kernel heap, then init and load it.
pub fn init() {
    let idt = Box::new({
        use arch::interrupt::irq::*;
        use consts::irq::*;
		use arch::gdt::DOUBLE_FAULT_IST_INDEX;
        
        let mut idt = Idt::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        idt[(T_IRQ0 + IRQ_COM1) as usize].set_handler_fn(com1_handler);
        idt[(T_IRQ0 + IRQ_COM2) as usize].set_handler_fn(com2_handler);
        idt[(T_IRQ0 + IRQ_KBD) as usize].set_handler_fn(keyboard_handler);
        idt[(T_IRQ0 + IRQ_TIMER) as usize].set_handler_fn(timer_handler);
        idt[T_SWITCH_TOU as usize].set_handler_fn(to_user_handler);
        idt[T_SWITCH_TOK as usize].set_handler_fn(to_kernel_handler);
        idt[T_SYSCALL as usize].set_handler_fn(syscall_handler);
        unsafe {
            idt.page_fault.set_handler_fn(page_fault_handler)
                .set_stack_index(DOUBLE_FAULT_IST_INDEX as u16);
        }
        idt
    });
    let idt = unsafe{ &*Box::into_raw(idt) };

    idt.load();
}