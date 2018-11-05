use alloc::boxed::Box;
use consts::MAX_CPU_NUM;
use core::fmt;
use core::fmt::Debug;
use spin::{Mutex, MutexGuard, Once};
use x86_64::{PrivilegeLevel, VirtAddr};
use x86_64::structures::gdt::*;
use x86_64::structures::tss::TaskStateSegment;

/// Alloc TSS & GDT at kernel heap, then init and load it.
/// The double fault stack will be allocated at kernel heap too.
pub fn init() {
    use x86_64::instructions::segmentation::set_cs;
    use x86_64::instructions::tables::load_tss;

    let double_fault_stack_top = Box::into_raw(Box::new([0u8; 4096])) as usize + 4096;
    debug!("Double fault stack top @ {:#x}", double_fault_stack_top);

    let tss = Box::new({
        let mut tss = TaskStateSegment::new();

        // 设置 Double Fault 时，自动切换栈的地址
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX]
            = VirtAddr::new(double_fault_stack_top as u64);

        tss
    });
    let tss = Box::into_raw(tss);

    let gdt = Box::new({
        let mut gdt = GlobalDescriptorTable::new();
        gdt.add_entry(KCODE);
        gdt.add_entry(UCODE);
        // KDATA use segment 0
        // gdt.add_entry(KDATA);
        gdt.add_entry(UDATA);
        gdt.add_entry(UCODE32);
        gdt.add_entry(UDATA32);
        gdt.add_entry(Descriptor::tss_segment(unsafe { &*tss }));
        gdt
    });
    let gdt = unsafe{ &*Box::into_raw(gdt) };
    gdt.load();

    unsafe {
        // reload code segment register
        set_cs(KCODE_SELECTOR);
        // load TSS
        load_tss(TSS_SELECTOR);
    }

    CPUS[super::cpu::id() as usize].call_once(||
        Mutex::new(Cpu { gdt, tss: unsafe { &mut *tss } }));
}

static CPUS: [Once<Mutex<Cpu>>; MAX_CPU_NUM] = [
    // TODO: More elegant ?
    Once::new(), Once::new(), Once::new(), Once::new(),
    Once::new(), Once::new(), Once::new(), Once::new(),
];

pub struct Cpu {
    gdt: &'static GlobalDescriptorTable,
    tss: &'static mut TaskStateSegment,
}

impl Cpu {
    pub fn current() -> MutexGuard<'static, Cpu> {
        CPUS[super::cpu::id()].try().unwrap().lock()
    }

    /// 设置从Ring3跳到Ring0时，自动切换栈的地址
    ///
    /// 每次进入用户态前，都要调用此函数，才能保证正确返回内核态
    pub fn set_ring0_rsp(&mut self, rsp: usize) {
        trace!("gdt.set_ring0_rsp: {:#x}", rsp);
        self.tss.privilege_stack_table[0] = VirtAddr::new(rsp as u64);
    }
}

pub const DOUBLE_FAULT_IST_INDEX: usize = 0;

// Copied from xv6 x86_64
const KCODE: Descriptor = Descriptor::UserSegment(0x0020980000000000);  // EXECUTABLE | USER_SEGMENT | PRESENT | LONG_MODE
const UCODE: Descriptor = Descriptor::UserSegment(0x0020F80000000000);  // EXECUTABLE | USER_SEGMENT | USER_MODE | PRESENT | LONG_MODE
const KDATA: Descriptor = Descriptor::UserSegment(0x0000920000000000);  // DATA_WRITABLE | USER_SEGMENT | PRESENT
const UDATA: Descriptor = Descriptor::UserSegment(0x0000F20000000000);  // DATA_WRITABLE | USER_SEGMENT | USER_MODE | PRESENT
// Copied from xv6
const UCODE32: Descriptor = Descriptor::UserSegment(0x00cffa00_0000ffff);
// EXECUTABLE | USER_SEGMENT | USER_MODE | PRESENT
const UDATA32: Descriptor = Descriptor::UserSegment(0x00cff200_0000ffff);  // EXECUTABLE | USER_SEGMENT | USER_MODE | PRESENT

pub const KCODE_SELECTOR: SegmentSelector = SegmentSelector::new(1, PrivilegeLevel::Ring0);
pub const UCODE_SELECTOR: SegmentSelector = SegmentSelector::new(2, PrivilegeLevel::Ring3);
pub const KDATA_SELECTOR: SegmentSelector = SegmentSelector::new(0, PrivilegeLevel::Ring0);
pub const UDATA_SELECTOR: SegmentSelector = SegmentSelector::new(3, PrivilegeLevel::Ring3);
pub const UCODE32_SELECTOR: SegmentSelector = SegmentSelector::new(4, PrivilegeLevel::Ring3);
pub const UDATA32_SELECTOR: SegmentSelector = SegmentSelector::new(5, PrivilegeLevel::Ring3);
pub const TSS_SELECTOR: SegmentSelector = SegmentSelector::new(6, PrivilegeLevel::Ring0);