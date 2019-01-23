pub fn halt() {
    unsafe { asm!("wfi" :::: "volatile") }
}

pub fn id() -> usize {
    // TODO: cpu id
    0
}

pub fn fence() {
    unsafe { asm!("dmb ish" ::: "memory"); }
}
