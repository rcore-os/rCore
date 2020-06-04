use bitflags::*;

pub const SIG_ERR: usize = usize::max_value() - 1;
pub const SIG_DFL: usize = 0;
pub const SIG_IGN: usize = 1;

// yet there's a bug because of mismatching bits: https://sourceware.org/bugzilla/show_bug.cgi?id=25657
pub type SigSet = u64; // just support 64bits size sigset

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SignalAction {
    pub handler: usize, // this field may be an union
    pub mask: SigSet,
    pub flags: u32,
    pub _restorer: usize,
}

#[repr(C)]
pub union SigInfoFields {
    pad: [u8; Self::PAD_SIZE],
    // TODO: fill this union
}

impl SigInfoFields {
    const PAD_SIZE: usize = 128 - 2 * core::mem::size_of::<i32>() - core::mem::size_of::<usize>();
}

impl Default for SigInfoFields {
    fn default() -> Self {
        SigInfoFields {
            pad: [0; Self::PAD_SIZE]
        }
    }
}

#[repr(C)]
pub struct SigInfo {
    pub signo: i32,
    pub errno: i32,
    pub code: i32,
    pub field: SigInfoFields,
}

bitflags! {
    pub struct SignalActionFlags : u32 {
        const NOCLDSTOP = 1;
        const NOCLDWAIT = 2;
        const SIGINFO = 4;
        const ONSTACK = 0x08000000;
        const RESTART = 0x10000000;
        const NODEFER = 0x40000000;
        const RESETHAND = 0x80000000;
        const RESTORER = 0x04000000;
    }
}
