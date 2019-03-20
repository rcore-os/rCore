use crate::memory::active_table;
use alloc::boxed::Box;
use alloc::vec::Vec;
use isomorphic_drivers::provider;
use rcore_memory::paging::PageTable;
use rcore_memory::PAGE_SIZE;

#[derive(Copy, Clone)]
pub struct Provider;

impl Provider {
    pub fn new() -> Box<Provider> {
        Box::new(Provider {})
    }
}

impl provider::Provider for Provider {
    /// Get page size
    fn get_page_size(&self) -> usize {
        PAGE_SIZE
    }

    // Translate virtual address to physical address
    fn translate_va(&self, va: usize) -> usize {
        active_table().get_entry(va).unwrap().target()
    }

    // Bulk translate virtual addresses to physical addresses for performance
    fn translate_vas(&self, vas: &[usize]) -> Vec<usize> {
        let mut result = Vec::new();
        for va in vas.iter() {
            result.push(active_table().get_entry(*va).unwrap().target());
        }
        result
    }
}
