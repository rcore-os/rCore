use alloc::boxed::Box;

use x86_64::{PrivilegeLevel, VirtAddr};
use x86_64::structures::gdt::*;
use x86_64::structures::tss::TaskStateSegment;

use crate::consts::MAX_CPU_NUM;

/// Init TSS & GDT.
pub fn init() {
    unsafe {
        CPUS[super::cpu::id()] = Some(Cpu::new());
        CPUS[super::cpu::id()].as_mut().unwrap().init();
    }
}

static mut CPUS: [Option<Cpu>; MAX_CPU_NUM] = [
    // TODO: More elegant ?
    None, None, None, None,
    None, None, None, None,
];

pub struct Cpu {
    gdt: GlobalDescriptorTable,
    tss: TaskStateSegment,
    double_fault_stack: [u8; 0x100],
}

impl Cpu {
    pub fn current() -> &'static mut Self {
        unsafe { CPUS[super::cpu::id()].as_mut().unwrap() }
    }

    fn new() -> Self {
        Cpu {
            gdt: GlobalDescriptorTable::new(),
            tss: TaskStateSegment::new(),
            double_fault_stack: [0u8; 0x100],
        }
    }

    unsafe fn init(&'static mut self) {
        use x86_64::instructions::segmentation::set_cs;
        use x86_64::instructions::tables::load_tss;

        // Set the stack when DoubleFault occurs
        let stack_top = VirtAddr::new(self.double_fault_stack.as_ptr() as u64 + 0x100);
        self.tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX] = stack_top;

        // GDT
        self.gdt.add_entry(KCODE);
        self.gdt.add_entry(UCODE);
        // KDATA use segment 0
        // self.gdt.add_entry(KDATA);
        self.gdt.add_entry(UDATA);
        self.gdt.add_entry(UCODE32);
        self.gdt.add_entry(UDATA32);
        self.gdt.add_entry(Descriptor::tss_segment(&self.tss));
        self.gdt.load();

        // reload code segment register
        set_cs(KCODE_SELECTOR);
        // load TSS
        load_tss(TSS_SELECTOR);
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