use core::fmt;
use core::fmt::Debug;
use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::SegmentSelector;
use x86_64::{PrivilegeLevel, VirtualAddress};
use spin::Once;
use alloc::boxed::Box;

/// Alloc TSS & GDT at kernel heap, then init and load it.
/// The double fault stack will be allocated at kernel heap too.
pub fn init() {
    use x86_64::structures::gdt::SegmentSelector;
    use x86_64::instructions::segmentation::set_cs;
    use x86_64::instructions::tables::load_tss;

    let double_fault_stack_top = Box::into_raw(Box::new([0u8; 4096])) as usize + 4096;

    let mut tss = Box::new({
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX]
            = VirtualAddress(double_fault_stack_top);
        tss
    });
    let tss = unsafe{ &*Box::into_raw(tss) };

    let mut code_selector = SegmentSelector(0);
    let mut tss_selector = SegmentSelector(0);
    let gdt = Box::new({
        let mut gdt = Gdt::new();
        code_selector =
        gdt.add_entry(KCODE);
        gdt.add_entry(UCODE);
        gdt.add_entry(KDATA);
        gdt.add_entry(UDATA);
        tss_selector = gdt.add_entry(Descriptor::tss_segment(&tss));
        gdt
    });
    let gdt = unsafe{ &*Box::into_raw(gdt) };
    gdt.load();

    unsafe {
        // reload code segment register
        set_cs(code_selector);
        // load TSS
        load_tss(tss_selector);
    }
}

pub const DOUBLE_FAULT_IST_INDEX: usize = 0;

// Copied from xv6 x86_64
const KCODE: Descriptor = Descriptor::UserSegment(0x0020980000000000);  // EXECUTABLE | USER_SEGMENT | PRESENT | LONG_MODE
const UCODE: Descriptor = Descriptor::UserSegment(0x0020F80000000000);  // EXECUTABLE | USER_SEGMENT | USER_MODE | PRESENT | LONG_MODE
const KDATA: Descriptor = Descriptor::UserSegment(0x0000920000000000);  // DATA_WRITABLE | USER_SEGMENT | PRESENT
const UDATA: Descriptor = Descriptor::UserSegment(0x0000F20000000000);  // DATA_WRITABLE | USER_SEGMENT | USER_MODE | PRESENT

pub struct Gdt {
    table: [u64; 8],
    next_free: usize,
}

impl Gdt {
    pub fn new() -> Gdt {
        Gdt {
            table: [0; 8],
            next_free: 1,
        }
    }

    pub fn add_entry(&mut self, entry: Descriptor) -> SegmentSelector {
        let index = match entry {
            Descriptor::UserSegment(value) => self.push(value),
            Descriptor::SystemSegment(value_low, value_high) => {
                let index = self.push(value_low);
                self.push(value_high);
                index
            }
        };
        SegmentSelector::new(index as u16, PrivilegeLevel::Ring0)
    }

    pub fn load(&'static self) {
        use x86_64::instructions::tables::{DescriptorTablePointer, lgdt};
        use core::mem::size_of;

        let ptr = DescriptorTablePointer {
            base: self.table.as_ptr() as u64,
            limit: (self.table.len() * size_of::<u64>() - 1) as u16,
        };

        unsafe { lgdt(&ptr) };
    }

    fn push(&mut self, value: u64) -> usize {
        if self.next_free < self.table.len() {
            let index = self.next_free;
            self.table[index] = value;
            self.next_free += 1;
            index
        } else {
            panic!("GDT full");
        }
    }
}

pub enum Descriptor {
    UserSegment(u64),
    SystemSegment(u64, u64),
}

impl Descriptor {
    pub fn tss_segment(tss: &'static TaskStateSegment) -> Descriptor {
        use core::mem::size_of;
        use bit_field::BitField;

        let ptr = tss as *const _ as u64;

        let mut low = DescriptorFlags::PRESENT.bits();
        // base
        low.set_bits(16..40, ptr.get_bits(0..24));
        low.set_bits(56..64, ptr.get_bits(24..32));
        // limit (the `-1` in needed since the bound is inclusive)
        low.set_bits(0..16, (size_of::<TaskStateSegment>() - 1) as u64);
        // type (0b1001 = available 64-bit tss)
        low.set_bits(40..44, 0b1001);

        let mut high = 0;
        high.set_bits(0..32, ptr.get_bits(32..64));

        Descriptor::SystemSegment(low, high)
    }
}

bitflags! {
    /// Reference: https://wiki.osdev.org/GDT
    struct DescriptorFlags: u64 {
        const ACCESSED          = 1 << 40;
        const DATA_WRITABLE     = 1 << 41;
        const CODE_READABLE     = 1 << 41;
        const CONFORMING        = 1 << 42;
        const EXECUTABLE        = 1 << 43;
        const USER_SEGMENT      = 1 << 44;
        const USER_MODE         = 1 << 45 | 1 << 46;
        const PRESENT           = 1 << 47;
        const LONG_MODE         = 1 << 53;
    }
}

impl Debug for Descriptor {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Descriptor::UserSegment(flags) => 
                write!(f, "UserSegment( {:?} )", DescriptorFlags{bits: *flags}),
            Descriptor::SystemSegment(low, high) =>
                write!(f, "SystemSegment{:?}", (low, high)),
        }
    }
}

pub mod test
{
    pub fn print_flags() {
        use super::*;
        // The following 4 GDT entries were copied from xv6 x86_64
        let list: [(&str, Descriptor); 4] = [
            ("KCODE", super::KCODE), // Code, DPL=0, R/X
            ("UCODE", super::UCODE), // Code, DPL=3, R/X
            ("KDATA", super::KDATA), // Data, DPL=0, W
            ("UDATA", super::UDATA), // Data, DPL=3, W
        ];
        // Let's see what that means
        println!("GDT Segments from xv6 x86_64:");
        for (name, desc) in list.iter() {
            println!("  {}: {:?}", name, desc);
        }
    }
}