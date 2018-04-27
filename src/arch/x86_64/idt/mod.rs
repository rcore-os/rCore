use self::idt::*;
use spin::Once;
use alloc::boxed::Box;

mod idt;

/// Alloc IDT at kernel heap, then init and load it.
pub fn init() {
    let idt = Box::new({
        use arch::interrupt::{handler::*, consts::*};
		use arch::gdt::DOUBLE_FAULT_IST_INDEX;
        
        let mut idt = Idt::new();
        idt[T_BRKPT].set_handler_fn(breakpoint);
        idt[T_PGFLT].set_handler_fn(page_fault);
        idt[T_GPFLT].set_handler_fn(general_protection_fault);
        idt[T_IRQ0 + IRQ_COM1].set_handler_fn(com1);
        idt[T_IRQ0 + IRQ_COM2].set_handler_fn(com2);
        idt[T_IRQ0 + IRQ_KBD].set_handler_fn(keyboard);
        idt[T_IRQ0 + IRQ_TIMER].set_handler_fn(timer);

        idt[T_SWITCH_TOU].set_handler_fn(to_user);
        idt[T_SWITCH_TOK].set_handler_fn(to_kernel)
            .set_flags(IdtFlags::PRESENT | IdtFlags::RING_3 | IdtFlags::INTERRUPT);
        idt[T_SYSCALL].set_handler_fn(syscall)
            .set_flags(IdtFlags::PRESENT | IdtFlags::RING_3 | IdtFlags::TRAP);

        unsafe {
            idt[T_DBLFLT].set_handler_fn(double_fault)
                .set_stack_index(DOUBLE_FAULT_IST_INDEX as u16);
        }
        idt
    });
    let idt = unsafe{ &*Box::into_raw(idt) };

    idt.load();
}