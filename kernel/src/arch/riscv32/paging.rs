use crate::consts::{KERNEL_P2_INDEX, RECURSIVE_INDEX};
// Depends on kernel
use crate::memory::{active_table, alloc_frame, dealloc_frame};
use riscv::addr::*;
use riscv::asm::{sfence_vma, sfence_vma_all};
use riscv::paging::{Mapper, PageTable as RvPageTable, PageTableEntry, PageTableFlags as EF, RecursivePageTable};
use riscv::paging::{FrameAllocator, FrameDeallocator};
use riscv::register::satp;
use ucore_memory::memory_set::*;
use ucore_memory::PAGE_SIZE;
use ucore_memory::paging::*;
use log::*;

/*
* @param:
*   Frame: page table root frame
* @brief:
*   setup page table in the frame
*/
// need 1 page
pub fn setup_page_table(frame: Frame) {
    let p2 = unsafe { &mut *(frame.start_address().as_u32() as *mut RvPageTable) };
    p2.zero();
    p2.set_recursive(RECURSIVE_INDEX, frame.clone());

    // Set kernel identity map
    // 0x10000000 ~ 1K area
    p2.map_identity(0x40, EF::VALID | EF::READABLE | EF::WRITABLE);
    // 0x80000000 ~ 12M area 
    p2.map_identity(KERNEL_P2_INDEX, EF::VALID | EF::READABLE | EF::WRITABLE | EF::EXECUTABLE);
    p2.map_identity(KERNEL_P2_INDEX + 1, EF::VALID | EF::READABLE | EF::WRITABLE | EF::EXECUTABLE);
    p2.map_identity(KERNEL_P2_INDEX + 2, EF::VALID | EF::READABLE | EF::WRITABLE | EF::EXECUTABLE);

    unsafe { satp::set(satp::Mode::Sv32, 0, frame); }
    sfence_vma_all();
    info!("setup init page table end");
}

pub struct ActivePageTable(RecursivePageTable<'static>);

pub struct PageEntry(PageTableEntry);

impl PageTable for ActivePageTable {
    /*
    * @param:
    *   addr: the virtual addr to be matched
    *   target: the physical addr to be matched with addr
    * @brief:
    *   map the virtual address 'addr' to the physical address 'target' in pagetable.
    * @retval:
    *   the matched PageEntry
    */
    fn map(&mut self, addr: usize, target: usize) -> &mut Entry {
        // the flag for the new page entry
        let flags = EF::VALID | EF::READABLE | EF::WRITABLE;
        // here page is for the virtual address while frame is for the physical, both of them is 4096 bytes align
        let page = Page::of_addr(VirtAddr::new(addr));
        let frame = Frame::of_addr(PhysAddr::new(target as u32));
        // map the page to the frame using FrameAllocatorForRiscv
        // we may need frame allocator to alloc frame for new page table(first/second)
        self.0.map_to(page, frame, flags, &mut FrameAllocatorForRiscv)
            .unwrap().flush();
        self.get_entry(addr).expect("fail to get entry")
    }

    /*
    * @param:
    *   addr: virtual address of which the mapped physical frame should be unmapped
    * @bridf:
    ^   unmap the virtual addresses' mapped physical frame
    */
    fn unmap(&mut self, addr: usize) {
        let page = Page::of_addr(VirtAddr::new(addr));
        let (frame, flush) = self.0.unmap(page).unwrap();
        flush.flush();
    }

    /*
    * @param:
    *   addr:input virtual address
    * @brief:
    *   get the pageEntry of 'addr'
    * @retval:
    *   a mutable PageEntry reference of 'addr'
    */
    fn get_entry(&mut self, addr: usize) -> Option<&mut Entry> {
        if unsafe { !(*ROOT_PAGE_TABLE)[addr >> 22].flags().contains(EF::VALID) } {
            return None;
        }
        let page = Page::of_addr(VirtAddr::new(addr));
        // ???
        let _ = self.0.translate_page(page);
        let entry_addr = ((addr >> 10) & ((1 << 22) - 4)) | (RECURSIVE_INDEX << 22);
        unsafe { Some(&mut *(entry_addr as *mut PageEntry)) }
    }
}

impl PageTableExt for ActivePageTable {}

// define the ROOT_PAGE_TABLE, and the virtual address of it?
const ROOT_PAGE_TABLE: *mut RvPageTable =
    (((RECURSIVE_INDEX << 10) | (RECURSIVE_INDEX + 1)) << 12) as *mut RvPageTable;

impl ActivePageTable {
    pub unsafe fn new() -> Self {
        ActivePageTable(RecursivePageTable::new(&mut *ROOT_PAGE_TABLE).unwrap())
    }
}
/// implementation for the Entry trait in /crate/memory/src/paging/mod.rs
impl Entry for PageEntry {
    fn update(&mut self) {
        let addr = VirtAddr::new((self as *const _ as usize) << 10);
        sfence_vma(0, addr);
    }
    fn accessed(&self) -> bool { self.0.flags().contains(EF::ACCESSED) }
    fn dirty(&self) -> bool { self.0.flags().contains(EF::DIRTY) }
    fn writable(&self) -> bool { self.0.flags().contains(EF::WRITABLE) }
    fn present(&self) -> bool { self.0.flags().contains(EF::VALID | EF::READABLE) }
    fn clear_accessed(&mut self) { self.as_flags().remove(EF::ACCESSED); }
    fn clear_dirty(&mut self) { self.as_flags().remove(EF::DIRTY); }
    fn set_writable(&mut self, value: bool) { self.as_flags().set(EF::WRITABLE, value); }
    fn set_present(&mut self, value: bool) { self.as_flags().set(EF::VALID | EF::READABLE, value); }
    fn target(&self) -> usize { self.0.addr().as_u32() as usize }
    fn set_target(&mut self, target: usize) {
        let flags = self.0.flags();
        let frame = Frame::of_addr(PhysAddr::new(target as u32));
        self.0.set(frame, flags);
    }
    fn writable_shared(&self) -> bool { self.0.flags().contains(EF::RESERVED1) }
    fn readonly_shared(&self) -> bool { self.0.flags().contains(EF::RESERVED2) }
    fn set_shared(&mut self, writable: bool) {
        let flags = self.as_flags();
        flags.set(EF::RESERVED1, writable);
        flags.set(EF::RESERVED2, !writable);
    }
    fn clear_shared(&mut self) { self.as_flags().remove(EF::RESERVED1 | EF::RESERVED2); }
    fn swapped(&self) -> bool { self.0.flags().contains(EF::RESERVED1) }
    fn set_swapped(&mut self, value: bool) { self.as_flags().set(EF::RESERVED1, value); }
    fn user(&self) -> bool { self.0.flags().contains(EF::USER) }
    fn set_user(&mut self, value: bool) { self.as_flags().set(EF::USER, value); }
    fn execute(&self) -> bool { self.0.flags().contains(EF::EXECUTABLE) }
    fn set_execute(&mut self, value: bool) { self.as_flags().set(EF::EXECUTABLE, value); }
    fn mmio(&self) -> bool { unimplemented!() }
    fn set_mmio(&mut self, value: bool) { unimplemented!() }
}

impl PageEntry {
    fn as_flags(&mut self) -> &mut EF {
        unsafe { &mut *(self as *mut _ as *mut EF) }
    }
}

#[derive(Debug)]
pub struct InactivePageTable0 {
    p2_frame: Frame,
}

impl InactivePageTable for InactivePageTable0 {
    type Active = ActivePageTable;

    fn new_bare() -> Self {
        let target = alloc_frame().expect("failed to allocate frame");
        let frame = Frame::of_addr(PhysAddr::new(target as u32));
        active_table().with_temporary_map(target, |_, table: &mut RvPageTable| {
            table.zero();
            table.set_recursive(RECURSIVE_INDEX, frame.clone());
        });
        InactivePageTable0 { p2_frame: frame }
    }

    fn map_kernel(&mut self) {
        let table = unsafe { &mut *ROOT_PAGE_TABLE };
        let e0 = table[0x40];
        let e1 = table[KERNEL_P2_INDEX];
        let e2 = table[KERNEL_P2_INDEX + 1];
        let e3 = table[KERNEL_P2_INDEX + 2];
        assert!(!e1.is_unused());
        assert!(!e2.is_unused());
        assert!(!e3.is_unused());

        self.edit(|_| {
            table[0x40] = e0;
            table[KERNEL_P2_INDEX].set(e1.frame(), EF::VALID | EF::GLOBAL);
            table[KERNEL_P2_INDEX + 1].set(e2.frame(), EF::VALID | EF::GLOBAL);
            table[KERNEL_P2_INDEX + 2].set(e3.frame(), EF::VALID | EF::GLOBAL);
        });
    }

    fn token(&self) -> usize {
        self.p2_frame.number() | (1 << 31) // as satp
    }

    unsafe fn set_token(token: usize) {
        asm!("csrw 0x180, $0" :: "r"(token) :: "volatile");
    }

    fn active_token() -> usize {
        satp::read().bits()
    }

    fn flush_tlb() {
        sfence_vma_all();
    }

    fn edit<T>(&mut self, f: impl FnOnce(&mut Self::Active) -> T) -> T {
        let target = satp::read().frame().start_address().as_u32() as usize;
        active_table().with_temporary_map(target, |active_table, p2_table: &mut RvPageTable| {
            let backup = p2_table[RECURSIVE_INDEX].clone();

            // overwrite recursive mapping
            p2_table[RECURSIVE_INDEX].set(self.p2_frame.clone(), EF::VALID);
            sfence_vma_all();

            // execute f in the new context
            let ret = f(active_table);

            // restore recursive mapping to original p2 table
            p2_table[RECURSIVE_INDEX] = backup;
            sfence_vma_all();

            ret
        })
    }
}

impl Drop for InactivePageTable0 {
    fn drop(&mut self) {
        info!("PageTable dropping: {:?}", self);
        dealloc_frame(self.p2_frame.start_address().as_u32() as usize);
    }
}

struct FrameAllocatorForRiscv;

impl FrameAllocator for FrameAllocatorForRiscv {
    fn alloc(&mut self) -> Option<Frame> {
        alloc_frame().map(|addr| Frame::of_addr(PhysAddr::new(addr as u32)))
    }
}

impl FrameDeallocator for FrameAllocatorForRiscv {
    fn dealloc(&mut self, frame: Frame) {
        dealloc_frame(frame.start_address().as_u32() as usize);
    }
}
