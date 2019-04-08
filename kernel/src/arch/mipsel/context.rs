use mips::registers::cp0;
use mips::tlb;

/// Saved registers on a trap.
#[derive(Clone)]
#[repr(C)]
pub struct TrapFrame {
    /// unused 16 bytes
    pub unused: [usize; 4],
    /// CP0 status register
    pub status: cp0::status::Status,
    /// CP0 cause register
    pub cause: cp0::cause::Cause,
    /// CP0 EPC register
    pub epc: usize,
    /// CP0 vaddr register
    pub vaddr: usize,
    /// HI/LO registers
    pub hi: usize,
    pub lo: usize,
    /// General registers
    pub at: usize,
    pub v0: usize,
    pub v1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
    pub t7: usize,
    pub s0: usize,
    pub s1: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub t8: usize,
    pub t9: usize,
    pub k0: usize,
    pub k1: usize,
    pub gp: usize,
    pub sp: usize,
    pub fp: usize,
    pub ra: usize,
    /// Reserve space for hartid
    pub _hartid: usize,
}

impl TrapFrame {
    /// Constructs TrapFrame for a new kernel thread.
    ///
    /// The new thread starts at function `entry` with an usize argument `arg`.
    /// The stack pointer will be set to `sp`.
    fn new_kernel_thread(entry: extern fn(usize) -> !, arg: usize, sp: usize) -> Self {
        use core::mem::zeroed;
        let mut tf: Self = unsafe { zeroed() };
        tf.a0 = arg;
        tf.sp = sp;
        tf.epc = entry as usize;
        tf.status = cp0::status::read();
        tf.status.set_kernel_mode();
        tf.status.set_ie();
        tf.status.set_exl();
        tf
    }

    /// Constructs TrapFrame for a new user thread.
    ///
    /// The new thread starts at `entry_addr`.
    /// The stack pointer will be set to `sp`.
    fn new_user_thread(entry_addr: usize, sp: usize) -> Self {
        use core::mem::zeroed;
        let mut tf: Self = unsafe { zeroed() };
        tf.sp = sp;
        tf.epc = entry_addr;
        tf.status = cp0::status::read();
        tf.status.set_user_mode();
        tf.status.set_ie();
        tf.status.set_exl();
        tf
    }
}

use core::fmt::{Debug, Formatter, Error};
impl Debug for TrapFrame {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_struct("TrapFrame")
            .field("status", &self.status.bits)
            .field("epc", &self.epc)
            .field("cause", &self.cause.bits)
            .field("vaddr", &self.vaddr)
            .finish()
    }
}

/// Kernel stack contents for a new thread
#[derive(Debug)]
#[repr(C)]
pub struct InitStack {
    context: ContextData,
    tf: TrapFrame,
}

impl InitStack {
    /// Push the InitStack on the stack and transfer to a Context.
    unsafe fn push_at(self, stack_top: usize) -> Context {
        let ptr = (stack_top as *mut Self).sub(1); //real kernel stack top
        *ptr = self;
        Context { sp: ptr as usize }
    }
}

extern "C" {
    fn trap_return();
}

/// Saved registers for kernel context switches.
#[derive(Debug, Default)]
#[repr(C)]
struct ContextData {
    /// Return address
    ra: usize,
    /// Page table token
    satp: usize,
    /// Callee-saved registers
    s: [usize; 12],
}

impl ContextData {
    fn new(satp: usize) -> Self {
        ContextData { ra: trap_return as usize, satp, ..ContextData::default() }
    }
}

/// Context of a kernel thread.
#[derive(Debug)]
#[repr(C)]
pub struct Context {
    /// The stack pointer of the suspended thread.
    /// A `ContextData` is stored here.
    sp: usize,
}

impl Context {
    /// Switch to another kernel thread.
    ///
    /// Push all callee-saved registers at the current kernel stack.
    /// Store current sp, switch to target.
    /// Pop all callee-saved registers, then return to the target.
    #[inline(always)]
    pub unsafe fn switch(&mut self, target: &mut Self) {
        extern "C" {
            fn switch_context(src: *mut Context, dst: *mut Context);
        }

        tlb::clear_all_tlb();
        switch_context(self as *mut Context, target as *mut Context);
    }

    /// Constructs a null Context for the current running thread.
    pub unsafe fn null() -> Self {
        Context { sp: 0 }
    }

    /// Constructs Context for a new kernel thread.
    ///
    /// The new thread starts at function `entry` with an usize argument `arg`.
    /// The stack pointer will be set to `kstack_top`.
    /// The SATP register will be set to `satp`.
    pub unsafe fn new_kernel_thread(entry: extern fn(usize) -> !, arg: usize, kstack_top: usize, satp: usize) -> Self {
        info!("New kernel thread @ {:x}, stack = {:x}", entry as usize, kstack_top);

        InitStack {
            context: ContextData::new(satp),
            tf: TrapFrame::new_kernel_thread(entry, arg, kstack_top),
        }.push_at(kstack_top)
    }

    /// Constructs Context for a new user thread.
    ///
    /// The new thread starts at `entry_addr`.
    /// The stack pointer of user and kernel mode will be set to `ustack_top`, `kstack_top`.
    /// The SATP register will be set to `satp`.
    pub unsafe fn new_user_thread(entry_addr: usize, ustack_top: usize, kstack_top: usize, _is32: bool, satp: usize) -> Self {
        info!("New user thread @ {:x}, stack = {:x}", entry_addr, kstack_top);

        InitStack {
            context: ContextData::new(satp),
            tf: TrapFrame::new_user_thread(entry_addr, ustack_top),
        }.push_at(kstack_top)
    }

    /// Fork a user process and get the new Context.
    ///
    /// The stack pointer in kernel mode will be set to `kstack_top`.
    /// The SATP register will be set to `satp`.
    /// All the other registers are same as the original.
    pub unsafe fn new_fork(tf: &TrapFrame, kstack_top: usize, satp: usize) -> Self {
        InitStack {
            context: ContextData::new(satp),
            tf: {
                let mut tf = tf.clone();
                // fork function's ret value, the new process is 0
                tf.v0 = 0;
                tf
            },
        }.push_at(kstack_top)
    }

    /// Fork a user thread and get the new Context.
    ///
    /// The stack pointer in kernel mode will be set to `kstack_top`.
    /// The SATP register will be set to `satp`.
    /// The new user stack will be set to `ustack_top`.
    /// The new thread pointer will be set to `tls`.
    /// All the other registers are same as the original.
    pub unsafe fn new_clone(tf: &TrapFrame, ustack_top: usize, kstack_top: usize, satp: usize, tls: usize) -> Self {
        InitStack {
            context: ContextData::new(satp),
            tf: {
                let mut tf = tf.clone();
                tf.sp = ustack_top;   // sp
                tf.v1 = tls;
                tf.v0 = 0;  // return value
                tf
            },
        }.push_at(kstack_top)
    }

    /// Used for getting the init TrapFrame of a new user context in `sys_exec`.
    pub unsafe fn get_init_tf(&self) -> TrapFrame {
        (*(self.sp as *const InitStack)).tf.clone()
    }
}
