use mips::registers;

/// Saved registers on a trap.
#[derive(Clone)]
#[repr(C)]
pub struct TrapFrame {
    /// CP0 status register
    pub status: u32,
    /// CP0 cause register
    pub cause: u32,
    /// CP0 EPC register
    pub epc: u32,
    /// CP0 vaddr register
    pub vaddr: u32,
    /// HI/LO registers
    pub hi: u32,
    pub lo: u32,
    /// General registers
    pub at: u32,
    pub v0: u32,
    pub v1: u32,
    pub a0: u32,
    pub a1: u32,
    pub a2: u32,
    pub a3: u32,
    pub t0: u32,
    pub t1: u32,
    pub t2: u32,
    pub t3: u32,
    pub t4: u32,
    pub t5: u32,
    pub t6: u32,
    pub t7: u32,
    pub s0: u32,
    pub s1: u32,
    pub s2: u32,
    pub s3: u32,
    pub s4: u32,
    pub s5: u32,
    pub s6: u32,
    pub s7: u32,
    pub t8: u32,
    pub t9: u32,
    pub k0: u32,
    pub k1: u32,
    pub gp: u32,
    pub sp: u32,
    pub fp: u32,
    pub ra: u32,
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
        tf.x[10] = arg; // a0
        tf.x[2] = sp;
        tf.sepc = entry as usize;
        tf.sstatus = sstatus::read();
        tf.sstatus.set_spie(true);
        tf.sstatus.set_sie(false);
        tf.sstatus.set_spp(sstatus::SPP::Supervisor);
        tf
    }

    /// Constructs TrapFrame for a new user thread.
    ///
    /// The new thread starts at `entry_addr`.
    /// The stack pointer will be set to `sp`.
    fn new_user_thread(entry_addr: usize, sp: usize) -> Self {
        use core::mem::zeroed;
        let mut tf: Self = unsafe { zeroed() };
        tf.x[2] = sp;
        tf.sepc = entry_addr;
        tf.sstatus = sstatus::read();
        tf.sstatus.set_spie(true);
        tf.sstatus.set_sie(false);
        tf.sstatus.set_spp(sstatus::SPP::User);
        tf
    }
}

use core::fmt::{Debug, Formatter, Error};
impl Debug for TrapFrame {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        struct Regs<'a>(&'a [usize; 32]);
        impl<'a> Debug for Regs<'a> {
            fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
                const REG_NAME: [&str; 32] = [
                    "zero", "ra", "sp", "gp", "tp", "t0", "t1", "t2",
                    "s0", "s1", "a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7",
                    "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11",
                    "t3", "t4", "t5", "t6"];
                f.debug_map().entries(REG_NAME.iter().zip(self.0)).finish()
            }
        }
        f.debug_struct("TrapFrame")
            .field("regs", &Regs(&self.x))
            .field("sstatus", &self.sstatus)
            .field("sepc", &self.sepc)
            .field("stval", &self.stval)
            .field("scause", &self.scause.cause())
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

extern {
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
    #[naked]
    #[inline(never)]
    pub unsafe extern fn switch(&mut self, _target: &mut Self) {
        #[cfg(target_arch = "riscv32")]
        asm!(r"
        .equ XLENB, 4
        .macro Load reg, mem
            lw \reg, \mem
        .endm
        .macro Store reg, mem
            sw \reg, \mem
        .endm");
        #[cfg(target_arch = "riscv64")]
        asm!(r"
        .equ XLENB, 8
        .macro Load reg, mem
            ld \reg, \mem
        .endm
        .macro Store reg, mem
            sd \reg, \mem
        .endm");
        asm!("
        // save from's registers
        addi  sp, sp, (-XLENB*14)
        Store sp, 0(a0)
        Store ra, 0*XLENB(sp)
        Store s0, 2*XLENB(sp)
        Store s1, 3*XLENB(sp)
        Store s2, 4*XLENB(sp)
        Store s3, 5*XLENB(sp)
        Store s4, 6*XLENB(sp)
        Store s5, 7*XLENB(sp)
        Store s6, 8*XLENB(sp)
        Store s7, 9*XLENB(sp)
        Store s8, 10*XLENB(sp)
        Store s9, 11*XLENB(sp)
        Store s10, 12*XLENB(sp)
        Store s11, 13*XLENB(sp)
        csrr  s11, satp
        Store s11, 1*XLENB(sp)

        // restore to's registers
        Load sp, 0(a1)
        Load s11, 1*XLENB(sp)
        csrw satp, s11
        Load ra, 0*XLENB(sp)
        Load s0, 2*XLENB(sp)
        Load s1, 3*XLENB(sp)
        Load s2, 4*XLENB(sp)
        Load s3, 5*XLENB(sp)
        Load s4, 6*XLENB(sp)
        Load s5, 7*XLENB(sp)
        Load s6, 8*XLENB(sp)
        Load s7, 9*XLENB(sp)
        Load s8, 10*XLENB(sp)
        Load s9, 11*XLENB(sp)
        Load s10, 12*XLENB(sp)
        Load s11, 13*XLENB(sp)
        addi sp, sp, (XLENB*14)

        Store zero, 0(a1)
        ret"
        : : : : "volatile" )
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
                tf.x[10] = 0; // a0
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
                tf.x[2] = ustack_top;   // sp
                tf.x[4] = tls; // tp
                tf.x[10] = 0;  // a0
                tf
            },
        }.push_at(kstack_top)
    }

    /// Used for getting the init TrapFrame of a new user context in `sys_exec`.
    pub unsafe fn get_init_tf(&self) -> TrapFrame {
        (*(self.sp as *const InitStack)).tf.clone()
    }
}
