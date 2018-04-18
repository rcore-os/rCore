use arch::driver::{acpi::ACPI_Result, apic::start_ap};
use memory::{MemoryController, PhysicalAddress};

extern {
    fn entryother_start();  // physical addr of entryother
    fn entryother_end();
}

const ENTRYOTHER_ADDR: u32 = 0x7000;

pub fn start_other_cores(acpi: &ACPI_Result, mc: &mut MemoryController) {
    mc.map_page_identity(ENTRYOTHER_ADDR as usize - 1);
    mc.map_page_identity(ENTRYOTHER_ADDR as usize);
    mc.map_page_identity(entryother_start as usize);
    mc.map_page_p2v(PhysicalAddress(0));
    copy_entryother();

    let args = unsafe{ &mut *(ENTRYOTHER_ADDR as *mut EntryArgs).offset(-1) };
    let page_table = unsafe{ *(0xFFFF_FFFF_FFFF_FFF8 as *const u32) } & 0xFFFF_F000;
    for i in 1 .. acpi.cpu_num {
        let apic_id = acpi.cpu_acpi_ids[i as usize];
        *args = EntryArgs {
            kstack: mc.alloc_stack(1).unwrap().top() as u64,
            page_table: page_table,
            stack: 0x8000, // just enough stack to get us to entry64mp
        };
        start_ap(apic_id, ENTRYOTHER_ADDR);
        loop{}
    }

}

fn copy_entryother() {
    use rlibc::memmove;
    let entryother_start = entryother_start as usize;
    let entryother_end = entryother_end as usize;
    let size = entryother_end - entryother_start;
    unsafe{ memmove(ENTRYOTHER_ADDR as *mut u8, entryother_start as *mut u8, size); }
    debug!("smp: copied entryother code to 0x7000");
}

#[repr(C)]
struct EntryArgs {
    kstack: u64,
    page_table: u32,
    stack: u32,
}