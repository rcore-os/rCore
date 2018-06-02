use arch::driver::{acpi::AcpiResult, apic::start_ap};
use memory::*;
use core::ptr::{read_volatile, write_volatile};
use x86_64::registers::control_regs::cr3;

extern {
    fn entryother_start();  // physical addr of entryother
    fn entryother_end();
}

const ENTRYOTHER_ADDR: u32 = 0x7000;

pub fn start_other_cores(acpi: &AcpiResult, ms: &mut MemorySet) {
    use consts::KERNEL_OFFSET;
    ms.push(MemoryArea::new_identity(ENTRYOTHER_ADDR as usize - 1, ENTRYOTHER_ADDR as usize + 1, MemoryAttr::default(), "entry_other"));
    ms.push(MemoryArea::new_identity(entryother_start as usize, entryother_start as usize + 1, MemoryAttr::default(), "entry_other"));
    ms.push(MemoryArea::new_kernel(KERNEL_OFFSET, KERNEL_OFFSET + 1, MemoryAttr::default(), "entry_other3"));
    copy_entryother();

    let args = unsafe{ &mut *(ENTRYOTHER_ADDR as *mut EntryArgs).offset(-1) };
    for i in 1 .. acpi.cpu_num {
        let apic_id = acpi.cpu_acpi_ids[i as usize];
        let ms = MemorySet::new(7);
        *args = EntryArgs {
            kstack: ms.kstack_top() as u64,
            page_table: ms._page_table_addr().0 as u32,
            stack: 0x8000, // just enough stack to get us to entry64mp
        };
        unsafe { MS = Some(ms); }
        start_ap(apic_id, ENTRYOTHER_ADDR);
        while unsafe { !read_volatile(&STARTED[i as usize]) } {}
    }
}

fn copy_entryother() {
    use rlibc::memmove;
    let entryother_start = entryother_start as usize;
    let entryother_end = entryother_end as usize;
    let size = entryother_end - entryother_start;
    assert!(size <= 0x1000, "entryother code is too large, not supported.");
    unsafe{ memmove(ENTRYOTHER_ADDR as *mut u8, entryother_start as *mut u8, size); }
    debug!("smp: copied entryother code to 0x7000");
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