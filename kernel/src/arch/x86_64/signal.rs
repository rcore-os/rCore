use crate::arch::interrupt::TrapFrame;

#[repr(C)]
#[derive(Clone, Debug)]
pub struct MachineContext {
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
    pub fpstate: usize,
    pub _reserved1: [usize; 8],
}

impl MachineContext {
    pub fn from_tf(tf: &TrapFrame) -> Self {
        MachineContext {
            r8: tf.r8,
            r9: tf.r9,
            r10: tf.r10,
            r11: tf.r11,
            r12: tf.r12,
            r13: tf.r13,
            r14: tf.r14,
            r15: tf.r15,
            rdi: tf.rdi,
            rsi: tf.rsi,
            rbp: tf.rbp,
            rbx: tf.rbx,
            rdx: tf.rdx,
            rax: tf.rax,
            rcx: tf.rcx,
            rsp: tf.rsp,
            rip: tf.rip,
            eflags: 0,
            cs: tf.cs as u16,
            gs: 0,
            fs: 0,
            _pad: 0,
            err: 0,
            trapno: 0,
            oldmask: 0,
            cr2: 0,
            fpstate: 0,
            _reserved1: [0; 8],
        }
    }
}