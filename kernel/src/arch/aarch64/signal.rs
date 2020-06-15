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
        todo!()
    }
}
