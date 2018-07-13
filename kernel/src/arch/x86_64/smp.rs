use arch::driver::{acpi::AcpiResult, apic::start_ap};
use consts::MAX_CPU_NUM;
use core::ptr::{read_volatile, write_volatile};
use memory::*;
use x86_64::registers::control::Cr3;

pub const ENTRYOTHER_ADDR: usize = 0x7000;

pub fn start_other_cores(acpi: &AcpiResult) {
    let args = unsafe { &mut *(0x8000 as *mut EntryArgs).offset(-1) };
    for i in 1 .. acpi.cpu_num {
        let apic_id = acpi.cpu_acpi_ids[i as usize];
        let ms = MemorySet::new();
        *args = EntryArgs {
            kstack: ms.kstack_top() as u64,
            page_table: Cr3::read().0.start_address().as_u64() as u32,
            stack: args as *const _ as u32, // just enough stack to get us to entry64mp
        };
        unsafe { MS = Some(ms); }
        start_ap(apic_id, ENTRYOTHER_ADDR as u32);
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

static mut STARTED: [bool; MAX_CPU_NUM] = [false; MAX_CPU_NUM];
static mut MS: Option<MemorySet> = None;

pub unsafe fn notify_started(cpu_id: u8) -> MemorySet {
    write_volatile(&mut STARTED[cpu_id as usize], true);
    let ms = MS.take().unwrap();
    ms.activate();
    ms
}