#[cfg(feature = "m_mode")]
use riscv::register::{
    mstatus as xstatus,
    mstatus::Mstatus as Xstatus,
    mcause::Mcause,
};
#[cfg(not(feature = "m_mode"))]
use riscv::register::{
    sstatus as xstatus,
    sstatus::Sstatus as Xstatus,
    mcause::Mcause,
};

#[derive(Clone)]
#[repr(C)]
pub struct TrapFrame {
    pub x: [usize; 32], // general registers
    pub sstatus: Xstatus, // Supervisor Status Register
    pub sepc: usize, // Supervisor exception program counter, save the trap virtual address (here is used to save the process program entry addr?)
    pub stval: usize, // Supervisor trap value
    pub scause: Mcause, // scause register: record the cause of exception/interrupt/trap
}

/// Generate the trapframe for building new thread in kernel
impl TrapFrame {
    /*
    * @param:
    *   entry: program entry for the thread
    *   arg: a0
    *   sp: stack top
    * @brief:
    *   generate a trapfram for building a new kernel thread
    * @retval:
    *   the trapframe for new kernel thread
    */
    fn new_kernel_thread(entry: extern fn(usize) -> !, arg: usize, sp: usize) -> Self {
        use core::mem::zeroed;
        let mut tf: Self = unsafe { zeroed() };
        tf.x[10] = arg; // a0
        tf.x[2] = sp;
        tf.sepc = entry as usize;
        tf.sstatus = xstatus::read();
        tf.sstatus.set_xpie(true);
        tf.sstatus.set_xie(false);
        #[cfg(feature = "m_mode")]
        tf.sstatus.set_mpp(xstatus::MPP::Machine);
        #[cfg(not(feature = "m_mode"))]
        tf.sstatus.set_spp(xstatus::SPP::Supervisor);
        tf
    }

    /*
    * @param:
    *   entry_addr: program entry for the thread
    *   sp: stack top
    * @brief:
    *   generate a trapfram for building a new user thread
    * @retval:
    *   the trapframe for new user thread
    */
    fn new_user_thread(entry_addr: usize, sp: usize) -> Self {
        use core::mem::zeroed;
        let mut tf: Self = unsafe { zeroed() };
        tf.x[2] = sp;
        tf.sepc = entry_addr;
        tf.sstatus = xstatus::read();
        tf.sstatus.set_xpie(true);
        tf.sstatus.set_xie(false);
        #[cfg(feature = "m_mode")]
        tf.sstatus.set_mpp(xstatus::MPP::User);
        #[cfg(not(feature = "m_mode"))]
        tf.sstatus.set_spp(xstatus::SPP::User);
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
            .field("scause", &self.scause)
            .finish()
    }
}

/// kernel stack contents for a new thread
#[derive(Debug)]
#[repr(C)]
pub struct InitStack {
    context: ContextData,
    tf: TrapFrame,
}

impl InitStack {
    /*
    * @param:
    *   stack_top: the pointer to kernel stack stop
    * @brief:
    *   save the InitStack on the kernel stack stop
    * @retval:
    *   a Context with ptr in it
    */
    unsafe fn push_at(self, stack_top: usize) -> Context {
        let ptr = (stack_top as *mut Self).offset(-1); //real kernel stack top
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
        // satp(asid) just like cr3, save the physical address for Page directory?
        ContextData { ra: __trapret as usize, satp, ..ContextData::default() }
    }
}

/// A struct only contain one usize element
#[derive(Debug)]
pub struct Context(usize);

impl Context {
    /// Switch to another kernel thread.
    ///
    /// Push all callee-saved registers at the current kernel stack.
    /// Store current sp, switch to target.
    /// Pop all callee-saved registers, then return to the target.
    #[naked]
    #[inline(never)]
    pub unsafe extern fn switch(&mut self, target: &mut Self) {
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

    /*
    * @brief:
    *   generate a null Context
    * @retval:
    *   a null Context
    */
    pub unsafe fn null() -> Self {
        Context(0)
    }

    /*
    * @param:
    *   entry: program entry for the thread
    *   arg: a0
    *   kstack_top: kernel stack top
    *   cr3: cr3 register, save the physical address of Page directory
    * @brief:
    *   generate the content of kernel stack for the new kernel thread and save it's address at kernel stack top - 1
    * @retval:
    *   a Context struct with the pointer to the kernel stack top - 1 as its only element
    */
    pub unsafe fn new_kernel_thread(entry: extern fn(usize) -> !, arg: usize, kstack_top: usize, cr3: usize) -> Self {
        InitStack {
            context: ContextData::new(cr3),
            tf: TrapFrame::new_kernel_thread(entry, arg, kstack_top),
        }.push_at(kstack_top)
    }

    /*
    * @param:
    *   entry_addr: program entry for the thread
    *   ustack_top: user stack top
    *   kstack_top: kernel stack top
    *   is32: whether the cpu is 32 bit or not
    *   cr3: cr3 register, save the physical address of Page directory
    * @brief:
    *   generate the content of kernel stack for the new user thread and save it's address at kernel stack top - 1
    * @retval:
    *   a Context struct with the pointer to the kernel stack top - 1 as its only element
    */
    pub unsafe fn new_user_thread(entry_addr: usize, ustack_top: usize, kstack_top: usize, is32: bool, cr3: usize) -> Self {
        InitStack {
            context: ContextData::new(cr3),
            tf: TrapFrame::new_user_thread(entry_addr, ustack_top),
        }.push_at(kstack_top)
    }

    /*
    * @param:
    *   TrapFrame: the trapframe of the forked process(thread)
    *   kstack_top: kernel stack top
    *   cr3: cr3 register, save the physical address of Page directory
    * @brief:
    *   fork and generate a new process(thread) Context according to the TrapFrame and save it's address at kernel stack top - 1
    * @retval:
    *   a Context struct with the pointer to the kernel stack top - 1 as its only element
    */
    pub unsafe fn new_fork(tf: &TrapFrame, kstack_top: usize, cr3: usize) -> Self {
        InitStack {
            context: ContextData::new(cr3),
            tf: {
                let mut tf = tf.clone();
                // fork function's ret value, the new process is 0
                tf.x[10] = 0; // a0
                tf
            },
        }.push_at(kstack_top)
    }
    /// Called at a new user context
    /// To get the init TrapFrame in sys_exec
    pub unsafe fn get_init_tf(&self) -> TrapFrame {
        (*(self.0 as *const InitStack)).tf.clone()
    }
}
