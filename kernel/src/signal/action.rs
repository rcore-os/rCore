use bitflags::*;

pub const SIG_ERR: usize = usize::max_value() - 1;
pub const SIG_DFL: usize = 0;
pub const SIG_IGN: usize = 1;

// yet there's a bug because of mismatching bits: https://sourceware.org/bugzilla/show_bug.cgi?id=25657
pub type Sigset = u64; // just support 64bits size sigset

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SigAction {
    pub handler: usize, // this field may be an union
    pub mask: Sigset,
    pub flags: u32,
    pub _restorer: usize,
}

bitflags! {
    pub struct Flags : u32 {
        const SA_NOCLDSTOP = 1;
        const SA_NOCLDWAIT = 2;
        const SA_SIGINFO = 4;
        const SA_ONSTACK = 0x08000000;
        const SA_RESTART = 0x10000000;
        const SA_NODEFER = 0x40000000;
        const SA_RESETHAND = 0x80000000;
        const SA_RESTORER = 0x04000000;
    }
}
