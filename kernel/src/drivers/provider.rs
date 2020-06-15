pub use crate::arch::paging::PageTableImpl;
use crate::memory::{alloc_frame_contiguous, dealloc_frame, phys_to_virt, virt_to_phys};
use isomorphic_drivers::provider;
use rcore_memory::PAGE_SIZE;

pub struct Provider;

impl provider::Provider for Provider {
    const PAGE_SIZE: usize = PAGE_SIZE;

    fn alloc_dma(size: usize) -> (usize, usize) {
        let paddr = virtio_dma_alloc(size / PAGE_SIZE);
        let vaddr = phys_to_virt(paddr);
        (vaddr, paddr)
    }

    fn dealloc_dma(vaddr: usize, size: usize) {
        let paddr = virt_to_phys(vaddr);
        for i in 0..size / PAGE_SIZE {
            dealloc_frame(paddr + i * PAGE_SIZE);
        }
    }
}

#[no_mangle]
extern "C" fn virtio_dma_alloc(pages: usize) -> PhysAddr {
    let paddr = alloc_frame_contiguous(pages, 0).unwrap();
    trace!("alloc DMA: paddr={:#x}, pages={}", paddr, pages);
    paddr
}

#[no_mangle]
extern "C" fn virtio_dma_dealloc(paddr: PhysAddr, pages: usize) -> i32 {
    for i in 0..pages {
        dealloc_frame(paddr + i * PAGE_SIZE);
    }
    trace!("dealloc DMA: paddr={:#x}, pages={}", paddr, pages);
    0
}

#[no_mangle]
extern "C" fn virtio_phys_to_virt(paddr: PhysAddr) -> VirtAddr {
    phys_to_virt(paddr)
}

#[no_mangle]
extern "C" fn virtio_virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
    virt_to_phys(vaddr)
}

type VirtAddr = usize;
type PhysAddr = usize;
