//! Trapframe and context definitions for aarch64.

/// TODO
#[repr(C)]
#[derive(Debug)]
pub struct TrapFrame {
    // TODO
}

///TODO
#[derive(Debug)]
pub struct Context {
    // TODO
}

impl Context {
    /// TODO
    #[inline(never)]
    pub unsafe extern fn switch(&mut self, target: &mut Self) {
        unimplemented!()
    }

    /// TODO
    pub unsafe fn null() -> Self {
        unimplemented!()
    }

    /// TODO
    pub unsafe fn new_kernel_thread(entry: extern fn(usize) -> !, arg: usize, kstack_top: usize, cr3: usize) -> Self {
        unimplemented!()
    }

    /// TODO
    pub unsafe fn new_user_thread(entry_addr: usize, ustack_top: usize, kstack_top: usize, is32: bool, cr3: usize) -> Self {
        unimplemented!()
    }

    /// TODO
    pub unsafe fn new_fork(tf: &TrapFrame, kstack_top: usize, cr3: usize) -> Self {
        unimplemented!()
    }
}

#[inline(always)]
pub unsafe fn enable() {
    unimplemented!()
}

#[inline(always)]
pub unsafe fn disable() {
    unimplemented!()
}

#[inline(always)]
pub unsafe fn disable_and_store() -> usize {
    unimplemented!()
}

#[inline(always)]
pub unsafe fn restore(flags: usize) {
    unimplemented!()
}
