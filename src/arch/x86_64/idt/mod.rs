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
        for i in 0u8..=255 {
            idt[i].set_handler_fn(unsafe { __vectors[i as usize] });
        }

        idt[T_SWITCH_TOK].set_flags(IdtFlags::PRESENT | IdtFlags::RING_3 | IdtFlags::INTERRUPT);
        // TODO: Enable interrupt during syscall
        idt[T_SYSCALL].set_flags(IdtFlags::PRESENT | IdtFlags::RING_3 | IdtFlags::INTERRUPT);
        idt[0x80].set_flags(IdtFlags::PRESENT | IdtFlags::RING_3 | IdtFlags::INTERRUPT);

        unsafe {
            idt[T_DBLFLT].set_stack_index(DOUBLE_FAULT_IST_INDEX as u16);
        }
        idt
    });
    let idt = unsafe{ &*Box::into_raw(idt) };

    idt.load();
}

extern {
    //noinspection RsStaticConstNaming
    static __vectors: [extern fn(); 256];
}
