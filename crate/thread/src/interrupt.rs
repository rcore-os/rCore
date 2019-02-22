//! Enable and disable interrupt for each architecture.

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub unsafe fn disable_and_store() -> usize {
    let rflags: usize;
    asm!("pushfq; popq $0; cli" : "=r"(rflags) ::: "volatile");
    rflags & (1 << 9)
}

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub unsafe fn restore(flags: usize) {
    if flags != 0 {
        asm!("sti" :::: "volatile");
    }
}

#[inline(always)]
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
pub unsafe fn disable_and_store() -> usize {
    if env!("M_MODE") == "1" {
        let mstatus: usize;
        asm!("csrci mstatus, 1 << 3" : "=r"(mstatus) ::: "volatile");
        mstatus & (1 << 3)
    } else {
        let sstatus: usize;
        asm!("csrci sstatus, 1 << 1" : "=r"(sstatus) ::: "volatile");
        sstatus & (1 << 1)
    }
}

#[inline(always)]
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
pub unsafe fn restore(flags: usize) {
    if env!("M_MODE") == "1" {
        asm!("csrs mstatus, $0" :: "r"(flags) :: "volatile");
    } else {
        asm!("csrs sstatus, $0" :: "r"(flags) :: "volatile");
    }
}

#[inline(always)]
#[cfg(target_arch = "aarch64")]
pub unsafe fn disable_and_store() -> usize {
    let daif: u32;
    asm!("mrs $0, DAIF; msr daifset, #2": "=r"(daif) ::: "volatile");
    daif as usize
}

#[inline(always)]
#[cfg(target_arch = "aarch64")]
pub unsafe fn restore(flags: usize) {
    asm!("msr DAIF, $0" :: "r"(flags as u32) :: "volatile");
}
