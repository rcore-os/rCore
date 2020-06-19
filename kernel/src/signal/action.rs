use crate::signal::Signal;
use bitflags::_core::fmt::Debug;
use bitflags::*;
use core::fmt::Formatter;

pub const SIG_ERR: usize = usize::max_value() - 1;
pub const SIG_DFL: usize = 0;
pub const SIG_IGN: usize = 1;

pub const SI_ASYNCNL: i32 = -60;
pub const SI_TKILL: i32 = -6;
pub const SI_SIGIO: i32 = -5;
pub const SI_ASYNCIO: i32 = -4;
pub const SI_MESGQ: i32 = -3;
pub const SI_TIMER: i32 = -2;
pub const SI_QUEUE: i32 = -1;
pub const SI_USER: i32 = 0;
pub const SI_KERNEL: i32 = 128;

// yet there's a bug because of mismatching bits: https://sourceware.org/bugzilla/show_bug.cgi?id=25657
// just support 64bits size sigset
#[derive(Default, Clone, Copy, Debug)]
#[repr(C)]
pub struct Sigset(u64);

impl Sigset {
    pub fn empty() -> Self {
        Sigset(0)
    }

    pub fn contains(&self, sig: Signal) -> bool {
        (self.0 >> sig as u64 & 1) != 0
    }

    pub fn add(&mut self, sig: Signal) {
        self.0 |= 1 << sig as u64;
    }
    pub fn add_set(&mut self, sigset: &Sigset) {
        self.0 |= sigset.0;
    }
    pub fn remove(&mut self, sig: Signal) {
        self.0 ^= self.0 & (1 << sig as u64);
    }
    pub fn remove_set(&mut self, sigset: &Sigset) {
        self.0 ^= self.0 & sigset.0;
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SignalAction {
    pub handler: usize, // this field may be an union
    pub flags: usize,
    pub restorer: usize,
    pub mask: Sigset,
}

impl Debug for SignalAction {
    fn fmt(&self, f: &mut Formatter) -> Result<(), core::fmt::Error> {
        f.debug_struct("signal action")
            .field("handler", &self.handler)
            .field("mask", &self.mask)
            .field("flags", &SignalActionFlags::from_bits_truncate(self.flags))
            .field("restorer", &self.restorer)
            .finish()
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union SiginfoFields {
    pad: [u8; Self::PAD_SIZE],
    // TODO: fill this union
}

impl SiginfoFields {
    const PAD_SIZE: usize = 128 - 2 * core::mem::size_of::<i32>() - core::mem::size_of::<usize>();
}

impl Default for SiginfoFields {
    fn default() -> Self {
        SiginfoFields {
            pad: [0; Self::PAD_SIZE],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Siginfo {
    pub signo: i32,
    pub errno: i32,
    pub code: i32,
    pub field: SiginfoFields,
}

bitflags! {
    pub struct SignalActionFlags : usize {
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
