//! TrapFrame and context definitions for aarch64.

#[repr(C)]
#[derive(Default, Debug, Copy, Clone)]
pub struct TrapFrame {
    pub elr: u64,
    pub spsr: u64,
    pub sp: u64,
    pub tpidr: u64,
    // pub q0to31: [u128; 32], // disable SIMD/FP registers
    pub x1to29: [u64; 29],
    pub __reserved: u64,
    pub x30: u64, // lr
    pub x0: u64,
}

///TODO
#[derive(Debug)]
pub struct Context {
    // TODO
}

impl Context {
    /// TODO
    #[inline(never)]
    pub unsafe extern "C" fn switch(&mut self, target: &mut Self) {
        unimplemented!()
    }

    /// TODO
    pub unsafe fn null() -> Self {
        unimplemented!()
    }

    /// TODO
    pub unsafe fn new_kernel_thread(
        entry: extern "C" fn(usize) -> !,
        arg: usize,
        kstack_top: usize,
        cr3: usize,
    ) -> Self {
        unimplemented!()
    }

    /// TODO
    pub unsafe fn new_user_thread(
        entry_addr: usize,
        ustack_top: usize,
        kstack_top: usize,
        is32: bool,
        cr3: usize,
    ) -> Self {
        unimplemented!()
    }

    /// TODO
    pub unsafe fn new_fork(tf: &TrapFrame, kstack_top: usize, cr3: usize) -> Self {
        unimplemented!()
    }
}
