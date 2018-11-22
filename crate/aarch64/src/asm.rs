use paging::PhysFrame;
use addr::{PhysAddr, VirtAddr};
use regs::*;

#[inline(always)]
pub fn tlb_invalidate_all() {
    unsafe {
        asm!(
            "dsb ishst
             tlbi vmalle1is
             dsb ish
             isb"
        );
    }
}

#[inline(always)]
pub fn tlb_invalidate(vaddr: VirtAddr) {
    unsafe {
        asm!(
            "dsb ishst
             tlbi vaae1is, $0
             dsb ish
             isb" :: "r"(vaddr.as_u64() >> 12)
        );
    }
}

/// Returns the current stack pointer.
#[inline(always)]
pub fn sp() -> *const u8 {
    let ptr: usize;
    unsafe {
        asm!("mov $0, sp" : "=r"(ptr));
    }

    ptr as *const u8
}

#[inline(always)]
pub unsafe fn get_pc() -> usize {
    let pc: usize;
    asm!("ADR $0, ." : "=r"(pc));
    pc
}

/// Returns the current exception level.
///
/// # Safety
/// This function should only be called when EL is >= 1.
#[inline(always)]
pub unsafe fn current_el() -> u8 {
    let el_reg: u64;
    asm!("mrs $0, CurrentEL" : "=r"(el_reg));
    ((el_reg & 0b1100) >> 2) as u8
}

#[inline(always)]
pub unsafe fn get_far() -> usize {
    let far: usize;
    asm!("mrs $0, far_el1" : "=r"(far));
    far
}

#[inline(always)]
pub unsafe fn get_ttbr0() -> usize {
    let ttbr0: usize;
    asm!("mrs $0, ttbr0_el1" : "=r"(ttbr0));
    ttbr0
}

#[inline(always)]
pub unsafe fn get_ttbr1() -> usize {
    let ttbr0: usize;
    asm!("mrs $0, ttbr1_el1" : "=r"(ttbr0));
    ttbr0
}

#[inline(always)]
pub fn address_translate(vaddr: usize) -> usize {
    let paddr: usize;
    unsafe {
        asm!("at S1E1R, $1; mrs $0, par_el1" : "=r"(paddr) : "r"(vaddr));
    }
    paddr
}

/// Returns the SPSel value.
#[inline(always)]
pub fn sp_sel() -> u8 {
    let ptr: u32;
    unsafe {
        asm!("mrs $0, SPSel" : "=r"(ptr));
    }

    (ptr & 1) as u8
}

/// Returns the core currently executing.
///
/// # Safety
///
/// This function should only be called when EL is >= 1.
pub unsafe fn affinity() -> usize {
    let x: usize;
    asm!("mrs     $0, mpidr_el1
          and     $0, $0, #3"
          : "=r"(x));

    x
}

pub fn wfi() {
    unsafe {
        asm!("wfi" :::: "volatile");
    }
}

/// The classic no-op
#[inline]
pub fn nop() {
    match () {
        #[cfg(target_arch = "aarch64")]
        () => unsafe { asm!("nop" :::: "volatile") },

        #[cfg(not(target_arch = "aarch64"))]
        () => unimplemented!(),
    }
}

/// Wait For Event
#[inline]
pub fn wfe() {
    match () {
        #[cfg(target_arch = "aarch64")]
        () => unsafe { asm!("wfe" :::: "volatile") },

        #[cfg(not(target_arch = "aarch64"))]
        () => unimplemented!(),
    }
}

/// Exception return
///
/// Will jump to wherever the corresponding link register points to, and
/// therefore never return.
#[inline]
pub fn eret() -> ! {
    use core;

    match () {
        #[cfg(target_arch = "aarch64")]
        () => unsafe {
            asm!("eret" :::: "volatile");
            core::intrinsics::unreachable()
        },

        #[cfg(not(target_arch = "aarch64"))]
        () => unimplemented!(),
    }
}

pub fn ttbr_el1_read(which: u8) -> (PhysFrame) {
    let baddr = match which {
        0 => TTBR0_EL1.get_baddr(),
        1 => TTBR1_EL1.get_baddr(),
        _ => 0,
    };
    PhysFrame::containing_address(PhysAddr::new(baddr))
}

pub fn ttbr_el1_write(which: u8, frame: PhysFrame) {
    let baddr = frame.start_address().as_u64();
    match which {
        0 => TTBR0_EL1.set_baddr(baddr),
        1 => TTBR1_EL1.set_baddr(baddr),
        _ => {}
    };
}
