use aarch64::{asm, regs::*};

pub fn halt() {
    asm::wfi();
}

pub fn id() -> usize {
    (MPIDR_EL1.get() & 3) as usize
}

pub unsafe fn exit_in_qemu(_error_code: u8) -> ! {
    unimplemented!()
}

pub unsafe fn reboot() -> ! {
    unimplemented!()
}
