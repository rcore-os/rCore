//! TrapFrame and context definitions for aarch64.

use aarch64::barrier;
use aarch64::paging::PhysFrame;
use aarch64::translation::{local_invalidate_tlb_all, ttbr_el1_read, ttbr_el1_write_asid};
use lazy_static::lazy_static;
use spin::Mutex;

#[repr(C)]
#[derive(Default, Debug, Copy, Clone)]
pub struct TrapFrame {
    pub elr: usize,
    pub spsr: usize,
    pub sp: usize,
    pub tpidr: usize, // currently unused
    // pub q0to31: [u128; 32], // disable SIMD/FP registers
    pub x1to29: [usize; 29],
    pub __reserved: usize,
    pub x30: usize, // lr
    pub x0: usize,
}

/// 用于在内核栈中构造新线程的中断帧
impl TrapFrame {
    fn new_kernel_thread(entry: extern "C" fn(usize) -> !, arg: usize, sp: usize) -> Self {
        use core::mem::zeroed;
        let mut tf: Self = unsafe { zeroed() };
        tf.x0 = arg;
        tf.sp = sp;
        tf.elr = entry as usize;
        tf.spsr = 0b1101_00_0101; // To EL 1, enable IRQ
        tf
    }
    pub fn new_user_thread(entry_addr: usize, sp: usize) -> Self {
        use core::mem::zeroed;
        let mut tf: Self = unsafe { zeroed() };
        tf.sp = sp;
        tf.elr = entry_addr;
        tf.spsr = 0b1101_00_0000; // To EL 0, enable IRQ
        tf
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
    unsafe fn push_at(self, stack_top: usize, ttbr: usize) -> Context {
        let ptr = (stack_top as *mut Self).offset(-1);
        *ptr = self;
        Context {
            stack_top: ptr as usize,
            ttbr: PhysFrame::of_addr(ttbr as u64),
            asid: Asid::default(),
        }
    }
}

extern "C" {
    fn __trapret();
}

#[derive(Debug, Default)]
#[repr(C)]
struct ContextData {
    x19to29: [usize; 11],
    lr: usize,
}

impl ContextData {
    fn new() -> Self {
        ContextData {
            lr: __trapret as usize,
            ..ContextData::default()
        }
    }
}

#[derive(Debug)]
pub struct Context {
    stack_top: usize,
    ttbr: PhysFrame,
    asid: Asid,
}

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
    unsafe extern "C" fn __switch(_self_stack: &mut usize, _target_stack: &mut usize) {
        asm!(
        "
        mov x10, #-(12 * 8)
        add x8, sp, x10
        str x8, [x0]
        stp x19, x20, [x8], #16     // store callee-saved registers
        stp x21, x22, [x8], #16
        stp x23, x24, [x8], #16
        stp x25, x26, [x8], #16
        stp x27, x28, [x8], #16
        stp x29, lr, [x8], #16

        ldr x8, [x1]
        ldp x19, x20, [x8], #16     // restore callee-saved registers
        ldp x21, x22, [x8], #16
        ldp x23, x24, [x8], #16
        ldp x25, x26, [x8], #16
        ldp x27, x28, [x8], #16
        ldp x29, lr, [x8], #16
        mov sp, x8

        str xzr, [x1]
        ret"
        : : : : "volatile" );
    }

    pub unsafe fn switch(&mut self, target: &mut Self) {
        self.ttbr = ttbr_el1_read(0);
        target.asid = ASID_ALLOCATOR.lock().alloc(target.asid);

        // with ASID we needn't flush TLB frequently
        ttbr_el1_write_asid(0, target.asid.value, target.ttbr);
        barrier::dsb(barrier::ISH);
        Self::__switch(&mut self.stack_top, &mut target.stack_top);
    }

    pub unsafe fn null() -> Self {
        Context {
            stack_top: 0,
            ttbr: PhysFrame::of_addr(0),
            asid: Asid::default(),
        }
    }

    pub unsafe fn new_kernel_thread(
        entry: extern "C" fn(usize) -> !,
        arg: usize,
        kstack_top: usize,
        ttbr: usize,
    ) -> Self {
        InitStack {
            context: ContextData::new(),
            tf: TrapFrame::new_kernel_thread(entry, arg, kstack_top),
        }
        .push_at(kstack_top, ttbr)
    }
    pub unsafe fn new_user_thread(
        entry_addr: usize,
        ustack_top: usize,
        kstack_top: usize,
        ttbr: usize,
    ) -> Self {
        InitStack {
            context: ContextData::new(),
            tf: TrapFrame::new_user_thread(entry_addr, ustack_top),
        }
        .push_at(kstack_top, ttbr)
    }
    pub unsafe fn new_fork(tf: &TrapFrame, kstack_top: usize, ttbr: usize) -> Self {
        InitStack {
            context: ContextData::new(),
            tf: {
                let mut tf = tf.clone();
                tf.x0 = 0;
                tf
            },
        }
        .push_at(kstack_top, ttbr)
    }
    pub unsafe fn new_clone(
        tf: &TrapFrame,
        ustack_top: usize,
        kstack_top: usize,
        ttbr: usize,
        tls: usize,
    ) -> Self {
        InitStack {
            context: ContextData::new(),
            tf: {
                let mut tf = tf.clone();
                tf.sp = ustack_top;
                tf.tpidr = tls;
                tf.x0 = 0;
                tf
            },
        }
        .push_at(kstack_top, ttbr)
    }
}

const ASID_MASK: u16 = 0xffff;

#[derive(Debug, Copy, Clone, Default)]
#[repr(C)]
struct Asid {
    value: u16,
    generation: u16,
}

struct AsidAllocator(Asid);

impl AsidAllocator {
    fn new() -> Self {
        AsidAllocator(Asid {
            value: 0,
            generation: 1,
        })
    }

    fn alloc(&mut self, old_asid: Asid) -> Asid {
        if self.0.generation == old_asid.generation {
            return old_asid;
        }

        if self.0.value == ASID_MASK {
            self.0.value = 0;
            self.0.generation = self.0.generation.wrapping_add(1);
            if self.0.generation == 0 {
                self.0.generation += 1;
            }
            local_invalidate_tlb_all();
        }
        self.0.value += 1;
        return self.0;
    }
}

lazy_static! {
    static ref ASID_ALLOCATOR: Mutex<AsidAllocator> = Mutex::new(AsidAllocator::new());
}
