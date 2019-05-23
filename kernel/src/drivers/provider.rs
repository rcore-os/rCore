use alloc::alloc::{alloc_zeroed, dealloc, Layout};

pub use crate::arch::paging::PageTableImpl;
use crate::consts::PHYSICAL_MEMORY_OFFSET;
use crate::memory::{alloc_frame, dealloc_frame, phys_to_virt, virt_to_phys};
use isomorphic_drivers::provider;
use rcore_memory::paging::PageTable;
use rcore_memory::PAGE_SIZE;

pub struct Provider;

impl provider::Provider for Provider {
    const PAGE_SIZE: usize = PAGE_SIZE;

    fn alloc_dma(size: usize) -> (usize, usize) {
        // TODO: allocate continuous pages
        let mut paddr = alloc_frame().unwrap();
        for i in 1..(size / PAGE_SIZE) {
            let paddr_new = alloc_frame().unwrap();
            assert_eq!(paddr - PAGE_SIZE, paddr_new);
            paddr = paddr_new;
        }
        let vaddr = phys_to_virt(paddr);
        (vaddr, paddr)
    }

    fn dealloc_dma(vaddr: usize, size: usize) {
        let paddr = virt_to_phys(vaddr);
        dealloc_frame(paddr);
    }
}
