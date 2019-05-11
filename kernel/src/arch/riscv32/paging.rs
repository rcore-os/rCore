use crate::consts::PHYSICAL_MEMORY_OFFSET;
#[cfg(target_arch = "riscv64")]
use crate::consts::RECURSIVE_INDEX;
// Depends on kernel
#[cfg(target_arch = "riscv64")]
use crate::consts::KERNEL_P4_INDEX;
use crate::memory::{alloc_frame, dealloc_frame, phys_to_virt};
use log::*;
use rcore_memory::paging::*;
use riscv::addr::*;
use riscv::asm::{sfence_vma, sfence_vma_all};
use riscv::paging::{FrameAllocator, FrameDeallocator};
use riscv::paging::{
    Mapper, PageTable as RvPageTable, PageTableEntry, PageTableFlags as EF, PageTableType,
    RecursivePageTable, TwoLevelPageTable,
};
use riscv::register::satp;

pub struct PageTableImpl {
    page_table: TwoLevelPageTable<'static>,
    root_frame: Frame,
    entry: PageEntry,
}

/// PageTableEntry: the contents of this entry.
/// Page: this entry is the pte of page `Page`.
pub struct PageEntry(&'static mut PageTableEntry, Page);

impl PageTable for PageTableImpl {
    fn map(&mut self, addr: usize, target: usize) -> &mut Entry {
        // map the 4K `page` to the 4K `frame` with `flags`
        let flags = EF::VALID | EF::READABLE | EF::WRITABLE;
        let page = Page::of_addr(VirtAddr::new(addr));
        let frame = Frame::of_addr(PhysAddr::new(target));
        // we may need frame allocator to alloc frame for new page table(first/second)
        self.page_table
            .map_to(page, frame, flags, &mut FrameAllocatorForRiscv)
            .unwrap()
            .flush();
        self.get_entry(addr).expect("fail to get entry")
    }

    fn unmap(&mut self, addr: usize) {
        let page = Page::of_addr(VirtAddr::new(addr));
        let (_, flush) = self.page_table.unmap(page).unwrap();
        flush.flush();
    }

    fn get_entry(&mut self, vaddr: usize) -> Option<&mut Entry> {
        let page = Page::of_addr(VirtAddr::new(vaddr));
        if let Ok(e) = self.page_table.ref_entry(page.clone()) {
            let e = unsafe { &mut *(e as *mut PageTableEntry) };
            self.entry = PageEntry(e, page);
            Some(&mut self.entry as &mut Entry)
        } else {
            None
        }
    }

    fn get_page_slice_mut<'a>(&mut self, addr: usize) -> &'a mut [u8] {
        let frame = self
            .page_table
            .translate_page(Page::of_addr(VirtAddr::new(addr)))
            .unwrap();
        let vaddr = frame.start_address().as_usize() + PHYSICAL_MEMORY_OFFSET;
        unsafe { core::slice::from_raw_parts_mut(vaddr as *mut u8, 0x1000) }
    }
}

/// implementation for the Entry trait in /crate/memory/src/paging/mod.rs
impl Entry for PageEntry {
    fn update(&mut self) {
        unsafe {
            sfence_vma(0, self.1.start_address().as_usize());
        }
    }
    fn accessed(&self) -> bool {
        self.0.flags().contains(EF::ACCESSED)
    }
    fn dirty(&self) -> bool {
        self.0.flags().contains(EF::DIRTY)
    }
    fn writable(&self) -> bool {
        self.0.flags().contains(EF::WRITABLE)
    }
    fn present(&self) -> bool {
        self.0.flags().contains(EF::VALID | EF::READABLE)
    }
    fn clear_accessed(&mut self) {
        self.0.flags_mut().remove(EF::ACCESSED);
    }
    fn clear_dirty(&mut self) {
        self.0.flags_mut().remove(EF::DIRTY);
    }
    fn set_writable(&mut self, value: bool) {
        self.0.flags_mut().set(EF::WRITABLE, value);
    }
    fn set_present(&mut self, value: bool) {
        self.0.flags_mut().set(EF::VALID | EF::READABLE, value);
    }
    fn target(&self) -> usize {
        self.0.addr().as_usize()
    }
    fn set_target(&mut self, target: usize) {
        let flags = self.0.flags();
        let frame = Frame::of_addr(PhysAddr::new(target));
        self.0.set(frame, flags);
    }
    fn writable_shared(&self) -> bool {
        self.0.flags().contains(EF::RESERVED1)
    }
    fn readonly_shared(&self) -> bool {
        self.0.flags().contains(EF::RESERVED2)
    }
    fn set_shared(&mut self, writable: bool) {
        let flags = self.0.flags_mut();
        flags.set(EF::RESERVED1, writable);
        flags.set(EF::RESERVED2, !writable);
    }
    fn clear_shared(&mut self) {
        self.0.flags_mut().remove(EF::RESERVED1 | EF::RESERVED2);
    }
    fn swapped(&self) -> bool {
        self.0.flags().contains(EF::RESERVED1)
    }
    fn set_swapped(&mut self, value: bool) {
        self.0.flags_mut().set(EF::RESERVED1, value);
    }
    fn user(&self) -> bool {
        self.0.flags().contains(EF::USER)
    }
    fn set_user(&mut self, value: bool) {
        self.0.flags_mut().set(EF::USER, value);
    }
    fn execute(&self) -> bool {
        self.0.flags().contains(EF::EXECUTABLE)
    }
    fn set_execute(&mut self, value: bool) {
        self.0.flags_mut().set(EF::EXECUTABLE, value);
    }
    fn mmio(&self) -> u8 {
        0
    }
    fn set_mmio(&mut self, _value: u8) {}
}

impl PageTableImpl {
    /// Unsafely get the current active page table.
    /// WARN: You MUST call `core::mem::forget` for it after use!
    pub unsafe fn active() -> Self {
        let frame = Frame::of_ppn(PageTableImpl::active_token() & 0x7fffffff);
        let table = frame.as_kernel_mut(PHYSICAL_MEMORY_OFFSET);
        PageTableImpl {
            page_table: TwoLevelPageTable::new(table, PHYSICAL_MEMORY_OFFSET),
            root_frame: frame,
            entry: unsafe { core::mem::uninitialized() },
        }
    }
}

impl PageTableExt for PageTableImpl {
    fn new_bare() -> Self {
        let target = alloc_frame().expect("failed to allocate frame");
        let frame = Frame::of_addr(PhysAddr::new(target));

        let table = unsafe { &mut *(phys_to_virt(target) as *mut RvPageTable) };
        table.zero();

        PageTableImpl {
            page_table: TwoLevelPageTable::new(table, PHYSICAL_MEMORY_OFFSET),
            root_frame: frame,
            entry: unsafe { core::mem::uninitialized() },
        }
    }

    fn map_kernel(&mut self) {
        info!("mapping kernel linear mapping");
        let table = unsafe {
            &mut *(phys_to_virt(self.root_frame.start_address().as_usize()) as *mut RvPageTable)
        };
        for i in 256..1024 {
            let flags =
                EF::VALID | EF::READABLE | EF::WRITABLE | EF::EXECUTABLE | EF::ACCESSED | EF::DIRTY;
            let frame = Frame::of_addr(PhysAddr::new((i << 22) - PHYSICAL_MEMORY_OFFSET));
            table[i].set(frame, flags);
        }
    }

    fn token(&self) -> usize {
        self.root_frame.number() | (1 << 31)
    }

    unsafe fn set_token(token: usize) {
        asm!("csrw satp, $0" :: "r"(token) :: "volatile");
    }

    fn active_token() -> usize {
        let mut token: usize = 0;
        unsafe {
            asm!("csrr $0, satp" : "=r"(token) ::: "volatile");
        }
        token
    }

    fn flush_tlb() {
        debug!("flushing token {:x}", Self::active_token());
        unsafe {
            sfence_vma_all();
        }
    }
}

impl Drop for PageTableImpl {
    fn drop(&mut self) {
        dealloc_frame(self.root_frame.start_address().as_usize());
    }
}

struct FrameAllocatorForRiscv;

impl FrameAllocator for FrameAllocatorForRiscv {
    fn alloc(&mut self) -> Option<Frame> {
        alloc_frame().map(|addr| Frame::of_addr(PhysAddr::new(addr)))
    }
}

impl FrameDeallocator for FrameAllocatorForRiscv {
    fn dealloc(&mut self, frame: Frame) {
        dealloc_frame(frame.start_address().as_usize());
    }
}
