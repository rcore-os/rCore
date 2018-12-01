//! Miscellaneous assembly instructions and functions

use paging::PhysFrame;
use addr::{PhysAddr, VirtAddr};
use regs::*;

/// Returns the current stack pointer.
#[inline(always)]
pub fn sp() -> *const u8 {
    let ptr: usize;
    unsafe {
        asm!("mov $0, sp" : "=r"(ptr));
    }

    ptr as *const u8
}

/// Returns the current point counter.
#[inline(always)]
pub unsafe fn get_pc() -> usize {
    let pc: usize;
    asm!("adr $0, ." : "=r"(pc));
    pc
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

/// Wait For Interrupt
#[inline]
pub fn wfi() {
    match () {
        #[cfg(target_arch = "aarch64")]
        () => unsafe { asm!("wfi" :::: "volatile") },

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

/// Invalidate all TLB entries.
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

/// Invalidate TLB entries that would be used to translate the specified address.
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

/// Invalidate all instruction caches in Inner Shareable domain to Point of Unification.
#[inline(always)]
pub fn flush_icache_all() {
    unsafe {
        asm!(
            "ic ialluis
             dsb ish
             isb"
        );
    }
}

/// Address Translate.
#[inline(always)]
pub fn address_translate(vaddr: usize) -> usize {
    let paddr: usize;
    unsafe {
        asm!("at S1E1R, $1; mrs $0, par_el1" : "=r"(paddr) : "r"(vaddr));
    }
    paddr
}

/// Read TTBRx_EL1 as PhysFrame
pub fn ttbr_el1_read(which: u8) -> PhysFrame {
    let baddr = match which {
        0 => TTBR0_EL1.get_baddr(),
        1 => TTBR1_EL1.get_baddr(),
        _ => 0,
    };
    PhysFrame::containing_address(PhysAddr::new(baddr))
}

/// Write TTBRx_EL1 from PhysFrame
pub fn ttbr_el1_write(which: u8, frame: PhysFrame) {
    let baddr = frame.start_address().as_u64();
    match which {
        0 => TTBR0_EL1.set_baddr(baddr),
        1 => TTBR1_EL1.set_baddr(baddr),
        _ => {}
    };
}

/// write TTBRx_EL1 from PhysFrame and ASID
pub fn ttbr_el1_write_asid(which: u8, asid: u16, frame: PhysFrame) {
    let baddr = frame.start_address().as_u64();
    match which {
        0 => TTBR0_EL1.write(TTBR0_EL1::ASID.val(asid as u64) + TTBR0_EL1::BADDR.val(baddr >> 1)),
        1 => TTBR1_EL1.write(TTBR1_EL1::ASID.val(asid as u64) + TTBR1_EL1::BADDR.val(baddr >> 1)),
        _ => {}
    };
}
