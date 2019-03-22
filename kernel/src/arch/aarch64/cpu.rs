pub fn halt() {
    unsafe { asm!("wfi" :::: "volatile") }
}

pub fn id() -> usize {
    // TODO: cpu id
    0
}

pub unsafe fn exit_in_qemu(error_code: u8) -> ! {
    unimplemented!()
}
