use x86_64;
use arch::driver::{apic::IOAPIC, pic};

pub mod handler;
pub mod consts;

pub use self::handler::TrapFrame;

impl TrapFrame {
    pub fn new_kernel_thread(entry: extern fn(), rsp: usize) -> Self {
        use arch::gdt;
        let mut tf = TrapFrame::default();
        tf.iret.cs = gdt::KCODE_SELECTOR.0 as usize;
        tf.iret.rip = entry as usize;
        tf.iret.ss = gdt::KDATA_SELECTOR.0 as usize;
        tf.iret.rsp = rsp;
        tf.iret.rflags = 0x282;
        tf
    }
    pub fn new_user_thread(entry_addr: usize, rsp: usize, is32: bool) -> Self {
        use arch::gdt;
        let mut tf = TrapFrame::default();
        tf.iret.cs = if is32 { gdt::UCODE32_SELECTOR.0 } else { gdt::UCODE_SELECTOR.0 } as usize;
        tf.iret.rip = entry_addr;
        tf.iret.ss = if is32 { gdt::UDATA32_SELECTOR.0 } else { gdt::UDATA_SELECTOR.0 } as usize;
        tf.iret.rsp = rsp;
        tf.iret.rflags = 0x282;
        tf
    }
}

#[inline(always)]
pub unsafe fn enable() {
    x86_64::instructions::interrupts::enable();
}

#[inline(always)]
pub unsafe fn disable() {
    x86_64::instructions::interrupts::disable();
}

#[inline(always)]
pub fn enable_irq(irq: u8) {
    if cfg!(feature = "use_apic") {
        IOAPIC.lock().enable(irq, 0);
    } else {
        pic::enable_irq(irq);
    }
}