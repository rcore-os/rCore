use lazy_static::lazy_static;
use x86_64::structures::idt::*;

pub fn init() {
    IDT.load();
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
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

        let ring3 = [Syscall32];

        let mut idt = InterruptDescriptorTable::new();
        let entries = unsafe{ &mut *(&mut idt as *mut _ as *mut [Entry<HandlerFunc>; 256]) };
        for i in 0..256 {
            let opt = entries[i].set_handler_fn(unsafe { transmute(__vectors[i]) });
            if ring3.contains(&(i as u8)) {
                opt.set_privilege_level(PrivilegeLevel::Ring3);
                opt.disable_interrupts(false);
            }
            if i == DoubleFault as usize {
                unsafe{ opt.set_stack_index(DOUBLE_FAULT_IST_INDEX as u16); }
            }
        }
        idt
    };
}

extern "C" {
    /// 中断向量表
    /// 符号定义在 [trap.asm](boot/trap.asm)
    //noinspection RsStaticConstNaming
    static __vectors: [extern "C" fn(); 256];
}
