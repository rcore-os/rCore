//! TrapFrame and context definitions for aarch64.

#[repr(C)]
#[derive(Default, Debug, Copy, Clone)]
pub struct TrapFrame {
    pub elr: usize,
    pub spsr: usize,
    pub sp: usize,
    pub tpidr: usize,
    // pub q0to31: [u128; 32], // disable SIMD/FP registers
    pub x1to29: [usize; 29],
    pub __reserved: usize,
    pub x30: usize, // lr
    pub x0: usize,
}

/// 用于在内核栈中构造新线程的中断帧
impl TrapFrame {
    fn new_kernel_thread(entry: extern fn(usize) -> !, arg: usize, sp: usize) -> Self {
        use core::mem::zeroed;
        let mut tf: Self = unsafe { zeroed() };
        tf.x0 = arg;
        tf.sp = sp;
        tf.elr = entry as usize;
        tf.spsr = 0b1101_00_0101; // To EL 1, enable IRQ
        tf
    }
    fn new_user_thread(entry_addr: usize, sp: usize) -> Self {
        use core::mem::zeroed;
        let mut tf: Self = unsafe { zeroed() };
        tf.sp = sp;
        tf.elr = entry_addr;
        tf.spsr = 0b1101_00_0000; // To EL 0, enable IRQ
        tf
    }
    pub fn is_user(&self) -> bool {
        unimplemented!()
    }
}

/// 新线程的内核栈初始内容
#[derive(Debug)]
#[repr(C)]
pub struct InitStack {
    context: ContextData,
    tf: TrapFrame,
}

impl InitStack {
    unsafe fn push_at(self, stack_top: usize) -> Context {
        let ptr = (stack_top as *mut Self).offset(-1);
        *ptr = self;
        Context(ptr as usize)
    }
}

extern {
    fn __trapret();
}

#[derive(Debug, Default)]
#[repr(C)]
struct ContextData {
    x19to29: [usize; 11],
    lr: usize,
    ttbr1: usize,
}

impl ContextData {
    fn new(ttbr1: usize) -> Self {
        ContextData { lr: __trapret as usize, ttbr1, ..ContextData::default() }
    }
}


#[derive(Debug)]
pub struct Context(usize);

impl Context {
    /// Switch to another kernel thread.
    ///
    /// Defined in `trap.S`.
    ///
    /// Push all callee-saved registers at the current kernel stack.
    /// Store current sp, switch to target.
    /// Pop all callee-saved registers, then return to the target.
    #[naked]
    #[inline(never)]
    pub unsafe extern fn switch(&mut self, target: &mut Self) {
        asm!(
        "
        mov x10, #-(13 * 8)
        add x8, sp, x10
        str x8, [x0]
        stp x19, x20, [x8], #16     // store callee-saved registers
        stp x21, x22, [x8], #16
        stp x23, x24, [x8], #16
        stp x25, x26, [x8], #16
        stp x27, x28, [x8], #16
        stp x29, lr, [x8], #16
        mrs x9, ttbr1_el1
        str x9, [x8], #8

        ldr x8, [x1]
        ldp x19, x20, [x8], #16     // restore callee-saved registers
        ldp x21, x22, [x8], #16
        ldp x23, x24, [x8], #16
        ldp x25, x26, [x8], #16
        ldp x27, x28, [x8], #16
        ldp x29, lr, [x8], #16
        ldr x9, [x8], #8
        mov sp, x8

        msr ttbr1_el1, x9           // set new page directory
        // TODO: with ASID we needn't flush TLB
        dsb ishst                   // ensure write has completed
        tlbi vmalle1is              // invalidate the TLB entry for the entry that changes
        dsb ish                     // ensure TLB invalidation is complete
        isb                         // synchronize context on this processor

        str xzr, [x1]
        ret"
        : : : : "volatile" );
    }

    pub unsafe fn null() -> Self {
        Context(0)
    }

    pub unsafe fn new_kernel_thread(entry: extern fn(usize) -> !, arg: usize, kstack_top: usize, ttbr: usize) -> Self {
        InitStack {
            context: ContextData::new(ttbr),
            tf: TrapFrame::new_kernel_thread(entry, arg, kstack_top),
        }.push_at(kstack_top)
    }
    pub unsafe fn new_user_thread(entry_addr: usize, ustack_top: usize, kstack_top: usize, is32: bool, ttbr: usize) -> Self {
        InitStack {
            context: ContextData::new(ttbr), // TODO: set ASID
            tf: TrapFrame::new_user_thread(entry_addr, ustack_top),
        }.push_at(kstack_top)
    }
    pub unsafe fn new_fork(tf: &TrapFrame, kstack_top: usize, ttbr: usize) -> Self {
        InitStack {
            context: ContextData::new(ttbr), // TODO: set ASID
            tf: {
                let mut tf = tf.clone();
                tf.x0 = 0;
                tf
            },
        }.push_at(kstack_top)
    }
}
