use arch::driver::{acpi::AcpiResult, apic::start_ap};
use memory::*;
use core::ptr::{read_volatile, write_volatile};
use x86_64::registers::control_regs::cr3;

const ENTRYOTHER_ADDR: u32 = 0x7000;

pub fn start_other_cores(acpi: &AcpiResult, ms: &mut MemorySet) {
    use consts::KERNEL_OFFSET;
    ms.push(MemoryArea::new_identity(ENTRYOTHER_ADDR as usize, ENTRYOTHER_ADDR as usize + 1, MemoryAttr::default().execute(), "entry_other.text"));
    ms.push(MemoryArea::new_kernel(KERNEL_OFFSET, KERNEL_OFFSET + 1, MemoryAttr::default(), "entry_other.ctrl"));

    let args = unsafe { &mut *(0x8000 as *mut EntryArgs).offset(-1) };
    for i in 1 .. acpi.cpu_num {
        let apic_id = acpi.cpu_acpi_ids[i as usize];
        let ms = MemorySet::new(7);
        *args = EntryArgs {
            kstack: ms.kstack_top() as u64,
            page_table: cr3().0 as u32,
            stack: args as *const _ as u32, // just enough stack to get us to entry64mp
        };
        unsafe { MS = Some(ms); }
        start_ap(apic_id, ENTRYOTHER_ADDR);
        while unsafe { !read_volatile(&STARTED[i as usize]) } {}
    }
}

#[repr(C)]
#[derive(Debug)]
struct EntryArgs {
    kstack: u64,
    page_table: u32,
    stack: u32,
}

use consts::MAX_CPU_NUM;
static mut STARTED: [bool; MAX_CPU_NUM] = [false; MAX_CPU_NUM];
static mut MS: Option<MemorySet> = None;

pub unsafe fn notify_started(cpu_id: u8) -> MemorySet {
    write_volatile(&mut STARTED[cpu_id as usize], true);
    MS.take().unwrap()
}