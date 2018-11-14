#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub unsafe fn disable_and_store() -> usize {
    let rflags: usize;
    asm!("pushfq; popq $0; cli" : "=r"(rflags));
    rflags & (1 << 9)
}

#[inline(always)]
#[cfg(target_arch = "riscv32")]
pub unsafe fn disable_and_store() -> usize {
    let sstatus: usize;
    asm!("csrrci $0, 0x100, 1" : "=r"(sstatus));
    sstatus & 1
}

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub unsafe fn restore(flags: usize) {
    if flags != 0 {
        asm!("sti");
    }
}

#[inline(always)]
#[cfg(target_arch = "riscv32")]
pub unsafe fn restore(flags: usize) {
    asm!("csrs 0x100, $0" :: "r"(flags));
}