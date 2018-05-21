#[derive(Debug, Clone, Default)]
pub struct TrapFrame {
    // Pushed by __alltraps at 'trap.asm'
    pub r15: usize,
    pub r14: usize,
    pub r13: usize,
    pub r12: usize,
    pub rbp: usize,
    pub rbx: usize,

    pub r11: usize,
    pub r10: usize,
    pub r9: usize,
    pub r8: usize,
    pub rsi: usize,
    pub rdi: usize,
    pub rdx: usize,
    pub rcx: usize,
    pub rax: usize,

    // Pushed by vector{i} at 'vector.asm'
    pub trap_num: usize,
    pub error_code: usize,

    // Pushed by CPU
    pub rip: usize,
    pub cs: usize,
    pub rflags: usize,

    // Pushed by CPU when Ring3->0
    pub rsp: usize,
    pub ss: usize,
}

/// 用于在内核栈中构造新线程的中断帧
impl TrapFrame {
    pub fn new_kernel_thread(entry: extern fn(), rsp: usize) -> Self {
        use arch::gdt;
        let mut tf = TrapFrame::default();
        tf.cs = gdt::KCODE_SELECTOR.0 as usize;
        tf.rip = entry as usize;
        tf.ss = gdt::KDATA_SELECTOR.0 as usize;
        tf.rsp = rsp;
        tf.rflags = 0x282;
        tf
    }
    pub fn new_user_thread(entry_addr: usize, rsp: usize, is32: bool) -> Self {
        use arch::gdt;
        let mut tf = TrapFrame::default();
        tf.cs = if is32 { gdt::UCODE32_SELECTOR.0 } else { gdt::UCODE_SELECTOR.0 } as usize;
        tf.rip = entry_addr;
        tf.ss = if is32 { gdt::UDATA32_SELECTOR.0 } else { gdt::UDATA_SELECTOR.0 } as usize;
        tf.rsp = rsp;
        tf.rflags = 0x282;
        tf
    }
}