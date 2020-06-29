use trapframe::{GeneralRegs, UserContext};

/// struct mcontext
#[repr(C)]
#[derive(Clone, Debug)]
pub struct MachineContext {
    // gregs
    pub r8: usize,
    pub r9: usize,
    pub r10: usize,
    pub r11: usize,
    pub r12: usize,
    pub r13: usize,
    pub r14: usize,
    pub r15: usize,
    pub rdi: usize,
    pub rsi: usize,
    pub rbp: usize,
    pub rbx: usize,
    pub rdx: usize,
    pub rax: usize,
    pub rcx: usize,
    pub rsp: usize,
    pub rip: usize,
    pub eflags: usize,
    pub cs: u16,
    pub gs: u16,
    pub fs: u16,
    pub _pad: u16,
    pub err: usize,
    pub trapno: usize,
    pub oldmask: usize,
    pub cr2: usize,
    // fpregs
    // TODO
    pub fpstate: usize,
    // reserved
    pub _reserved1: [usize; 8],
}

impl MachineContext {
    pub fn from_tf(tf: &UserContext) -> Self {
        Self {
            r8: tf.general.r8,
            r9: tf.general.r9,
            r10: tf.general.r10,
            r11: tf.general.r11,
            r12: tf.general.r12,
            r13: tf.general.r13,
            r14: tf.general.r14,
            r15: tf.general.r15,
            rdi: tf.general.rdi,
            rsi: tf.general.rsi,
            rbp: tf.general.rbp,
            rbx: tf.general.rbx,
            rdx: tf.general.rdx,
            rax: tf.general.rax,
            rcx: tf.general.rcx,
            rsp: tf.general.rsp,
            rip: tf.general.rip,
            eflags: tf.general.rflags,
            cs: 0,
            gs: 0,
            fs: 0,
            _pad: 0,
            err: tf.error_code,
            trapno: tf.trap_num,
            oldmask: 0,
            cr2: 0,
            fpstate: 0,
            _reserved1: [0; 8],
        }
    }

    pub fn fill_tf(&self, ctx: &mut UserContext) {
        ctx.general.rax = self.rax;
        ctx.general.rbx = self.rbx;
        ctx.general.rcx = self.rcx;
        ctx.general.rdx = self.rdx;
        ctx.general.rsi = self.rsi;
        ctx.general.rdi = self.rdi;
        ctx.general.rbp = self.rbp;
        ctx.general.rsp = self.rsp;
        ctx.general.r8 = self.r8;
        ctx.general.r9 = self.r9;
        ctx.general.r10 = self.r10;
        ctx.general.r11 = self.r11;
        ctx.general.r12 = self.r12;
        ctx.general.r13 = self.r13;
        ctx.general.r14 = self.r14;
        ctx.general.r15 = self.r15;
        ctx.general.rip = self.rip;
        ctx.general.rflags = self.eflags;
        ctx.trap_num = self.trapno;
        ctx.error_code = self.err;
    }
}
