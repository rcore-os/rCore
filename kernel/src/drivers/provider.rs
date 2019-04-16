use alloc::alloc::{alloc_zeroed, dealloc, Layout};

use isomorphic_drivers::provider;
use rcore_memory::paging::PageTable;
use rcore_memory::PAGE_SIZE;

use crate::memory::active_table;

pub struct Provider;

impl provider::Provider for Provider {
    const PAGE_SIZE: usize = PAGE_SIZE;

    fn alloc_dma(size: usize) -> (usize, usize) {
        let layout = Layout::from_size_align(size, PAGE_SIZE).unwrap();
        let vaddr = unsafe { alloc_zeroed(layout) } as usize;
        let paddr = active_table().get_entry(vaddr).unwrap().target();
        (vaddr, paddr)
    }

    fn dealloc_dma(vaddr: usize, size: usize) {
        let layout = Layout::from_size_align(size, PAGE_SIZE).unwrap();
        unsafe { dealloc(vaddr as *mut u8, layout) }
    }
}
