use crate::arch::consts::PHYSICAL_MEMORY_OFFSET;
use acpi::{search_for_rsdp_bios, AcpiHandler, PhysicalMapping};
use core::ptr::NonNull;

struct Handler;

impl AcpiHandler for Handler {
    fn map_physical_region<T>(
        &mut self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<T> {
        PhysicalMapping {
            physical_start: physical_address,
            virtual_start: NonNull::new((physical_address + PHYSICAL_MEMORY_OFFSET) as *mut T)
                .unwrap(),
            region_length: size,
            mapped_length: size,
        }
    }

    fn unmap_physical_region<T>(&mut self, region: PhysicalMapping<T>) {
        // do nothing
    }
}

pub fn init() {
    let mut handler = Handler {};
    let res = unsafe { search_for_rsdp_bios(&mut handler) };
    debug!("ACPI {:#x?}", res);
}
