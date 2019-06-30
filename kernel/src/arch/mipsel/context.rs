use mips::registers::cp0;
use mips::tlb;

/// Saved registers on a trap.
#[derive(Clone)]
#[repr(C)]
pub struct TrapFrame {
    /// Non-zero if the kernel stack is not 16-byte-aligned
    pub unaligned_kstack: usize,
    /// unused 12 bytes
    pub unused: [usize; 3],
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
    /// Floating-point registers (contains garbage if no FP support present)
    pub f0: usize,
    pub f1: usize,
    pub f2: usize,
    pub f3: usize,
    pub f4: usize,
    pub f5: usize,
    pub f6: usize,
    pub f7: usize,
    pub f8: usize,
    pub f9: usize,
    pub f10: usize,
    pub f11: usize,
    pub f12: usize,
    pub f13: usize,
    pub f14: usize,
    pub f15: usize,
    pub f16: usize,
    pub f17: usize,
    pub f18: usize,
    pub f19: usize,
    pub f20: usize,
    pub f21: usize,
    pub f22: usize,
    pub f23: usize,
    pub f24: usize,
    pub f25: usize,
    pub f26: usize,
    pub f27: usize,
    pub f28: usize,
    pub f29: usize,
    pub f30: usize,
    pub f31: usize,
    /// Reserved
    pub reserved: usize,
    pub __padding: [usize; 2],
}

impl TrapFrame {
    /// Constructs TrapFrame for a new kernel thread.
    ///
    /// The new thread starts at function `entry` with an usize argument `arg`.
    /// The stack pointer will be set to `sp`.
    fn new_kernel_thread(entry: extern "C" fn(usize) -> !, arg: usize, sp: usize) -> Self {
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
    pub fn new_user_thread(entry_addr: usize, sp: usize) -> Self {
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

use core::fmt::{Debug, Error, Formatter};
impl Debug for TrapFrame {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_struct("TrapFrame")
            .field("status", &self.status.bits)
            .field("epc", &self.epc)
            .field("cause", &self.cause.bits)
            .field("vaddr", &self.vaddr)
            .field("at", &self.at)
            .field("v0", &self.v0)
            .field("v1", &self.v1)
            .field("a0", &self.a0)
            .field("a1", &self.a1)
            .field("a2", &self.a2)
            .field("a3", &self.a3)
            .field("t0", &self.t0)
            .field("t1", &self.t1)
            .field("t2", &self.t2)
            .field("t3", &self.t3)
            .field("t4", &self.t4)
            .field("t5", &self.t5)
            .field("t6", &self.t6)
            .field("t7", &self.t7)
            .field("s0", &self.s0)
            .field("s1", &self.s1)
            .field("s2", &self.s2)
            .field("s3", &self.s3)
            .field("s4", &self.s4)
            .field("s5", &self.s5)
            .field("s6", &self.s6)
            .field("s7", &self.s7)
            .field("t8", &self.t8)
            .field("t9", &self.t9)
            .field("k0", &self.k0)
            .field("k1", &self.k1)
            .field("gp", &self.gp)
            .field("sp", &self.sp)
            .field("fp", &self.fp)
            .field("ra", &self.ra)
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
    fn _cur_tls();
}

/// Saved registers for kernel context switches.
#[derive(Debug, Default)]
#[repr(C)]
struct ContextData {
    /// Return address
    ra: usize,
    /// Page table token
    satp: usize,
    /// s[0] = TLS
    /// s[1] = reserved
    /// s[2..11] = Callee-saved registers
    s: [usize; 12],
    __padding: [usize; 2],
}

impl ContextData {
    fn new(satp: usize, tls: usize) -> Self {
        let mut context = ContextData {
            ra: trap_return as usize,
            satp: satp,
            ..ContextData::default()
        };
        context.s[0] = tls;
        context
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
    pub unsafe fn new_kernel_thread(
        entry: extern "C" fn(usize) -> !,
        arg: usize,
        kstack_top: usize,
        satp: usize,
    ) -> Self {
        info!(
            "New kernel thread @ {:x}, stack = {:x}",
            entry as usize, kstack_top
        );

        InitStack {
            context: ContextData::new(satp, 0),
            tf: TrapFrame::new_kernel_thread(entry, arg, kstack_top),
        }
        .push_at(kstack_top)
    }

    /// Constructs Context for a new user thread.
    ///
    /// The new thread starts at `entry_addr`.
    /// The stack pointer of user and kernel mode will be set to `ustack_top`, `kstack_top`.
    /// The SATP register will be set to `satp`.
    pub unsafe fn new_user_thread(
        entry_addr: usize,
        ustack_top: usize,
        kstack_top: usize,
        satp: usize,
    ) -> Self {
        info!(
            "New user thread @ {:x}, stack = {:x}",
            entry_addr, kstack_top
        );

        InitStack {
            context: ContextData::new(satp, 0),
            tf: TrapFrame::new_user_thread(entry_addr, ustack_top),
        }
        .push_at(kstack_top)
    }

    /// Fork a user process and get the new Context.
    ///
    /// The stack pointer in kernel mode will be set to `kstack_top`.
    /// The SATP register will be set to `satp`.
    /// All the other registers are same as the original.
    pub unsafe fn new_fork(tf: &TrapFrame, kstack_top: usize, satp: usize) -> Self {
        let tls = *(_cur_tls as *const usize);
        InitStack {
            context: ContextData::new(satp, tls),
            tf: {
                let mut tf = tf.clone();
                // fork function's ret value, the new process is 0
                tf.v0 = 0;
                tf
            },
        }
        .push_at(kstack_top)
    }

    /// Fork a user thread and get the new Context.
    ///
    /// The stack pointer in kernel mode will be set to `kstack_top`.
    /// The SATP register will be set to `satp`.
    /// The new user stack will be set to `ustack_top`.
    /// The new thread pointer will be set to `tls`.
    /// All the other registers are same as the original.
    pub unsafe fn new_clone(
        tf: &TrapFrame,
        ustack_top: usize,
        kstack_top: usize,
        satp: usize,
        tls: usize,
    ) -> Self {
        InitStack {
            context: ContextData::new(satp, tls),
            tf: {
                let mut tf = tf.clone();
                tf.sp = ustack_top; // sp
                tf.v0 = 0; // return value
                tf
            },
        }
        .push_at(kstack_top)
    }
}
