use core::fmt;
use core::default::Default;

#[derive(Clone)]
#[repr(C)]
pub struct FpState([u8; 16+512]);

impl fmt::Debug for FpState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "fpstate")
    }
}

impl Default for FpState {
    fn default() -> Self {
        FpState([0u8; 16+512])
    }
}


#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct TrapFrame {
    // fpstate needs to be 16-byte aligned
    // so we reserve some space here and save the offset
    // the read fpstate begin from fpstate[offset]
    pub fpstate_offset: usize,
    pub fpstate: FpState,
    // Pushed by __alltraps at 'trap.asm'
    pub fsbase: usize,

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
    fn new_kernel_thread(entry: extern fn(usize) -> !, arg: usize, rsp: usize) -> Self {
        use crate::arch::gdt;
        let mut tf = TrapFrame::default();
        tf.rdi = arg;
        tf.cs = gdt::KCODE_SELECTOR.0 as usize;
        tf.rip = entry as usize;
        tf.ss = gdt::KDATA_SELECTOR.0 as usize;
        tf.rsp = rsp;
        tf.rflags = 0x282;
        tf.fpstate_offset = 16; // skip restoring for first time
        tf
    }
    fn new_user_thread(entry_addr: usize, rsp: usize, is32: bool) -> Self {
        use crate::arch::gdt;
        let mut tf = TrapFrame::default();
        tf.cs = if is32 { gdt::UCODE32_SELECTOR.0 } else { gdt::UCODE_SELECTOR.0 } as usize;
        tf.rip = entry_addr;
        tf.ss = if is32 { gdt::UDATA32_SELECTOR.0 } else { gdt::UDATA_SELECTOR.0 } as usize;
        tf.rsp = rsp;
        tf.rflags = 0x282;
        tf.fpstate_offset = 16; // skip restoring for first time
        tf
    }
    pub fn is_user(&self) -> bool {
        self.cs & 0x3 == 0x3
    }
}

#[derive(Debug, Default)]
#[repr(C)]
struct ContextData {
    cr3: usize,
    r15: usize,
    r14: usize,
    r13: usize,
    r12: usize,
    rbp: usize,
    rbx: usize,
    rip: usize,
}

impl ContextData {
    fn new(cr3: usize) -> Self {
        ContextData { rip: forkret as usize, cr3, ..ContextData::default() }
    }
}

/// 新线程的内核栈初始内容
#[derive(Debug)]
#[repr(C)]
struct InitStack {
    context: ContextData,
    trapret: usize,
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
    fn trap_ret();
}

/// The entry point of new thread
extern fn forkret() {
    // Will return to `trapret`
}

#[derive(Debug)]
pub struct Context(usize);

impl Context {
    /// Switch to another kernel thread.
    ///
    /// Defined in `trap.asm`.
    ///
    /// Push all callee-saved registers at the current kernel stack.
    /// Store current rsp, switch to target.
    /// Pop all callee-saved registers, then return to the target.
    #[naked]
    #[inline(never)]
    pub unsafe extern fn switch(&mut self, _target: &mut Self) {
        asm!(
        "
        // push rip (by caller)

        // Save old callee-save registers
        push rbx
        push rbp
        push r12
        push r13
        push r14
        push r15
        mov r15, cr3
        push r15

        // Switch stacks
        mov [rdi], rsp      // rdi = from_rsp
        mov rsp, [rsi]      // rsi = to_rsp

        // Save old callee-save registers
        pop r15
        mov cr3, r15
        pop r15
        pop r14
        pop r13
        pop r12
        pop rbp
        pop rbx

        // pop rip
        ret"
        : : : : "intel" "volatile" )
    }

    pub unsafe fn null() -> Self {
        Context(0)
    }

    pub unsafe fn new_kernel_thread(entry: extern fn(usize) -> !, arg: usize, kstack_top: usize, cr3: usize) -> Self {
        InitStack {
            context: ContextData::new(cr3),
            trapret: trap_ret as usize,
            tf: TrapFrame::new_kernel_thread(entry, arg, kstack_top),
        }.push_at(kstack_top)
    }
    pub unsafe fn new_user_thread(entry_addr: usize, ustack_top: usize, kstack_top: usize, is32: bool, cr3: usize) -> Self {
        InitStack {
            context: ContextData::new(cr3),
            trapret: trap_ret as usize,
            tf: TrapFrame::new_user_thread(entry_addr, ustack_top, is32),
        }.push_at(kstack_top)
    }
    pub unsafe fn new_fork(tf: &TrapFrame, kstack_top: usize, cr3: usize) -> Self {
        InitStack {
            context: ContextData::new(cr3),
            trapret: trap_ret as usize,
            tf: {
                let mut tf = tf.clone();
                tf.rax = 0;
                // skip syscall inst;
                tf.rip = tf.rip + 2;
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
