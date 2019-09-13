use crate::memory::phys_to_virt;
use acpi::{parse_rsdp, AcpiHandler, PhysicalMapping};
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
            virtual_start: NonNull::new(phys_to_virt(physical_address) as *mut T)
                .unwrap(),
            region_length: size,
            mapped_length: size,
        }
    }

    fn unmap_physical_region<T>(&mut self, region: PhysicalMapping<T>) {
        // do nothing
    }
}

pub fn init(rsdp_addr: usize) {
    let res = parse_rsdp(&mut Handler, rsdp_addr);
    if let Ok(acpi) = res {
        debug!("ACPI {:#x?}", acpi);
    }
}
