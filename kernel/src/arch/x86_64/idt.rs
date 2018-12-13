use x86_64::structures::idt::*;
use lazy_static::lazy_static;

pub fn init() {
    IDT.load();
}

lazy_static! {
    static ref IDT: Idt = {
        use crate::arch::interrupt::consts::*;
		use crate::arch::gdt::DOUBLE_FAULT_IST_INDEX;
        use x86_64::PrivilegeLevel;
        use core::mem::transmute;

        // 这里主要利用了x86_64库提供的IDT结构
        // 它进行了完善的封装，有强类型约束
        // 然而这里我们需要绕过一些限制，例如：
        // * 依赖于 "x86-interrupt" 函数ABI，而我们的是裸函数
        // * 某些保留中断号不允许设置，会触发panic
        // 于是下面用了一些trick绕过了它们

        let ring3 = [T_SWITCH_TOK, T_SYSCALL, T_SYSCALL32];

        let mut idt = Idt::new();
        let entries = unsafe{ &mut *(&mut idt as *mut _ as *mut [IdtEntry<HandlerFunc>; 256]) };
        for i in 0..256 {
            let mut opt = entries[i].set_handler_fn(unsafe { transmute(__vectors[i]) });
            if ring3.contains(&(i as u8)) {
                opt.set_privilege_level(PrivilegeLevel::Ring3);
                opt.disable_interrupts(false);
            }
            if i == T_DBLFLT as usize {
                unsafe{ opt.set_stack_index(DOUBLE_FAULT_IST_INDEX as u16); }
            }
        }
        idt
    };
}

extern {
    /// 中断向量表
    /// 符号定义在 [trap.asm](boot/trap.asm)
    //noinspection RsStaticConstNaming
    static __vectors: [extern fn(); 256];
}
