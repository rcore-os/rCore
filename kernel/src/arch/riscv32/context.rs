use super::super::riscv::register::*;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct TrapFrame {
    pub x: [usize; 32],
    pub sstatus: sstatus::Sstatus,
    pub sepc: usize,
    pub sbadaddr: usize,
    pub scause: scause::Scause,
}

/// 用于在内核栈中构造新线程的中断帧
impl TrapFrame {
    fn new_kernel_thread(entry: extern fn(usize) -> !, arg: usize, sp: usize) -> Self {
        use core::mem::zeroed;
        let mut tf: Self = unsafe { zeroed() };
        tf.x[10] = arg; // a0
        tf.x[2] = sp;
        tf.sepc = entry as usize;
        tf.sstatus = sstatus::read();
        tf.sstatus.set_spie(true);
        tf.sstatus.set_sie(false);
        tf.sstatus.set_spp(sstatus::SPP::Supervisor);
        tf
    }
    fn new_user_thread(entry_addr: usize, sp: usize) -> Self {
        use core::mem::zeroed;
        let mut tf: Self = unsafe { zeroed() };
        tf.x[2] = sp;
        tf.sepc = entry_addr;
        tf.sstatus = sstatus::read();
        tf.sstatus.set_spie(false);     // Enable interrupt
        tf.sstatus.set_spp(sstatus::SPP::User);
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
    ra: usize,
    satp: usize,
    s: [usize; 12],
}

impl ContextData {
    fn new(satp: usize) -> Self {
        ContextData { ra: __trapret as usize, satp, ..ContextData::default() }
    }
}

#[derive(Debug)]
pub struct Context(usize);

impl Context {
    /// Switch to another kernel thread.
    ///
    /// Defined in `trap.asm`.
    ///
    /// Push all callee-saved registers at the current kernel stack.
    /// Store current sp, switch to target.
    /// Pop all callee-saved registers, then return to the target.
    #[naked]
    #[inline(never)]
    pub unsafe extern fn switch(&mut self, target: &mut Self) {
        asm!(
        "
        // save from's registers
        addi sp, sp, -4*14
        sw sp, 0(a0)
        sw ra, 0*4(sp)
        sw s0, 2*4(sp)
        sw s1, 3*4(sp)
        sw s2, 4*4(sp)
        sw s3, 5*4(sp)
        sw s4, 6*4(sp)
        sw s5, 7*4(sp)
        sw s6, 8*4(sp)
        sw s7, 9*4(sp)
        sw s8, 10*4(sp)
        sw s9, 11*4(sp)
        sw s10, 12*4(sp)
        sw s11, 13*4(sp)
        csrrs s11, 0x180, x0 // satp
        sw s11, 1*4(sp)

        // restore to's registers
        lw sp, 0(a1)
        lw s11, 1*4(sp)
        csrrw x0, 0x180, s11 // satp
        lw ra, 0*4(sp)
        lw s0, 2*4(sp)
        lw s1, 3*4(sp)
        lw s2, 4*4(sp)
        lw s3, 5*4(sp)
        lw s4, 6*4(sp)
        lw s5, 7*4(sp)
        lw s6, 8*4(sp)
        lw s7, 9*4(sp)
        lw s8, 10*4(sp)
        lw s9, 11*4(sp)
        lw s10, 12*4(sp)
        lw s11, 13*4(sp)
        addi sp, sp, 4*14

        sw zero, 0(a1)
        ret"
        : : : : "volatile" )
    }

    pub unsafe fn null() -> Self {
        Context(0)
    }

    pub unsafe fn new_kernel_thread(entry: extern fn(usize) -> !, arg: usize, kstack_top: usize, cr3: usize) -> Self {
        InitStack {
            context: ContextData::new(cr3),
            tf: TrapFrame::new_kernel_thread(entry, arg, kstack_top),
        }.push_at(kstack_top)
    }
    pub unsafe fn new_user_thread(entry_addr: usize, ustack_top: usize, kstack_top: usize, is32: bool, cr3: usize) -> Self {
        InitStack {
            context: ContextData::new(cr3),
            tf: TrapFrame::new_user_thread(entry_addr, ustack_top),
        }.push_at(kstack_top)
    }
    pub unsafe fn new_fork(tf: &TrapFrame, kstack_top: usize, cr3: usize) -> Self {
        InitStack {
            context: ContextData::new(cr3),
            tf: {
                let mut tf = tf.clone();
                tf.x[10] = 0; // a0
                tf
            },
        }.push_at(kstack_top)
    }
}