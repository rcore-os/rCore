pub fn halt() {
    unsafe { asm!("wfi" :::: "volatile") }
}

pub fn id() -> usize {
    // TODO: cpu id
    0
}