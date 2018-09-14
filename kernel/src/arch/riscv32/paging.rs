use consts::{KERNEL_PML4, RECURSIVE_PAGE_PML4};
// Depends on kernel
use memory::{active_table, alloc_frame, alloc_stack, dealloc_frame};
use super::riscv::addr::*;
use super::riscv::asm::{sfence_vma, sfence_vma_all};
use super::riscv::paging::{Mapper, PageTable as RvPageTable, PageTableEntry, PageTableFlags as EF, RecursivePageTable};
use super::riscv::paging::{FrameAllocator, FrameDeallocator};
use super::riscv::register::satp;
use ucore_memory::memory_set::*;
use ucore_memory::PAGE_SIZE;
use ucore_memory::paging::*;

// need 1 page
pub fn setup_page_table(frame: Frame) {
    let p2 = unsafe { &mut *(frame.start_address().as_u32() as *mut RvPageTable) };
    p2.zero();
    p2.set_recursive(RECURSIVE_PAGE_PML4, frame.clone());

    // Set kernel identity map
    // 0x10000000 ~ 1K area
    p2.map_identity(0x40, EF::VALID | EF::READABLE | EF::WRITABLE);
    // 0x80000000 ~ 8K area
    p2.map_identity(KERNEL_PML4, EF::VALID | EF::READABLE | EF::WRITABLE | EF::EXECUTABLE);
    p2.map_identity(KERNEL_PML4 + 1, EF::VALID | EF::READABLE | EF::WRITABLE | EF::EXECUTABLE);

    use super::riscv::register::satp;
    unsafe { satp::set(satp::Mode::Sv32, 0, frame); }
    sfence_vma_all();
    info!("setup init page table end");
}

pub struct ActivePageTable(RecursivePageTable<'static>);

pub struct PageEntry(PageTableEntry);

impl PageTable for ActivePageTable {
    type Entry = PageEntry;

    fn map(&mut self, addr: usize, target: usize) -> &mut PageEntry {
        let flags = EF::VALID | EF::READABLE | EF::WRITABLE;
        let page = Page::of_addr(VirtAddr::new(addr));
        let frame = Frame::of_addr(PhysAddr::new(target as u32));
        self.0.map_to(page, frame, flags, &mut FrameAllocatorForRiscv)
            .unwrap().flush();
        self.get_entry(addr)
    }

    fn unmap(&mut self, addr: usize) {
        let page = Page::of_addr(VirtAddr::new(addr));
        let (frame, flush) = self.0.unmap(page).unwrap();
        flush.flush();
    }

    fn get_entry(&mut self, addr: usize) -> &mut PageEntry {
        let page = Page::of_addr(VirtAddr::new(addr));
        let _ = self.0.translate_page(page);
        let entry_addr = ((addr >> 10) & 0x003ffffc) | (RECURSIVE_PAGE_PML4 << 22);
        unsafe { &mut *(entry_addr as *mut PageEntry) }
    }

    fn get_page_slice_mut<'a, 'b>(&'a mut self, addr: usize) -> &'b mut [u8] {
        use core::slice;
        unsafe { slice::from_raw_parts_mut((addr & !(PAGE_SIZE - 1)) as *mut u8, PAGE_SIZE) }
    }

    fn read(&mut self, addr: usize) -> u8 {
        unsafe { *(addr as *const u8) }
    }

    fn write(&mut self, addr: usize, data: u8) {
        unsafe { *(addr as *mut u8) = data; }
    }
}

const ROOT_PAGE_TABLE: *mut RvPageTable =
    (((RECURSIVE_PAGE_PML4 << 10) | (RECURSIVE_PAGE_PML4 + 1)) << 12) as *mut RvPageTable;

impl ActivePageTable {
    pub unsafe fn new() -> Self {
        ActivePageTable(RecursivePageTable::new(&mut *ROOT_PAGE_TABLE).unwrap())
    }
    fn with_temporary_map(&mut self, frame: &Frame, f: impl FnOnce(&mut ActivePageTable, &mut RvPageTable)) {
        // Create a temporary page
        let page = Page::of_addr(VirtAddr::new(0xcafebabe));
        assert!(self.0.translate_page(page).is_none(), "temporary page is already mapped");
        // Map it to table
        self.map(page.start_address().as_usize(), frame.start_address().as_u32() as usize);
        // Call f
        let table = unsafe { &mut *(page.start_address().as_usize() as *mut _) };
        f(self, table);
        // Unmap the page
        self.unmap(0xcafebabe);
    }
}

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
    fn swapped(&self) -> bool { unimplemented!() }
    fn set_swapped(&mut self, value: bool) { unimplemented!() }
    fn user(&self) -> bool { self.0.flags().contains(EF::USER) }
    fn set_user(&mut self, value: bool) { self.as_flags().set(EF::USER, value); }
    fn execute(&self) -> bool { self.0.flags().contains(EF::EXECUTABLE) }
    fn set_execute(&mut self, value: bool) { self.as_flags().set(EF::EXECUTABLE, value); }
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

    fn new() -> Self {
        let mut pt = Self::new_bare();
        pt.map_kernel();
        pt
    }

    fn new_bare() -> Self {
        let frame = Self::alloc_frame().map(|target| Frame::of_addr(PhysAddr::new(target as u32)))
            .expect("failed to allocate frame");
        active_table().with_temporary_map(&frame, |_, table: &mut RvPageTable| {
            table.zero();
            table.set_recursive(RECURSIVE_PAGE_PML4, frame.clone());
        });
        InactivePageTable0 { p2_frame: frame }
    }

    fn edit(&mut self, f: impl FnOnce(&mut Self::Active)) {
        active_table().with_temporary_map(&satp::read().frame(), |active_table, p2_table: &mut RvPageTable| {
            let backup = p2_table[RECURSIVE_PAGE_PML4].clone();

            // overwrite recursive mapping
            p2_table[RECURSIVE_PAGE_PML4].set(self.p2_frame.clone(), EF::VALID);
            sfence_vma_all();

            // execute f in the new context
            f(active_table);

            // restore recursive mapping to original p4 table
            p2_table[RECURSIVE_PAGE_PML4] = backup;
            sfence_vma_all();
        });
    }

    unsafe fn activate(&self) {
        let old_frame = satp::read().frame();
        let new_frame = self.p2_frame.clone();
        debug!("switch table {:x?} -> {:x?}", old_frame, new_frame);
        if old_frame != new_frame {
            satp::set(satp::Mode::Sv32, 0, new_frame);
            sfence_vma_all();
        }
    }

    unsafe fn with(&self, f: impl FnOnce()) {
        let old_frame = satp::read().frame();
        let new_frame = self.p2_frame.clone();
        debug!("switch table {:x?} -> {:x?}", old_frame, new_frame);
        if old_frame != new_frame {
            satp::set(satp::Mode::Sv32, 0, new_frame);
            sfence_vma_all();
        }
        f();
        debug!("switch table {:x?} -> {:x?}", new_frame, old_frame);
        if old_frame != new_frame {
            satp::set(satp::Mode::Sv32, 0, old_frame);
            sfence_vma_all();
        }
    }

    fn token(&self) -> usize {
        self.p2_frame.number() | (1 << 31) // as satp
    }

    fn alloc_frame() -> Option<usize> {
        alloc_frame()
    }

    fn dealloc_frame(target: usize) {
        dealloc_frame(target)
    }

    fn alloc_stack() -> Stack {
        alloc_stack()
    }
}

impl InactivePageTable0 {
    fn map_kernel(&mut self) {
        let table = unsafe { &mut *ROOT_PAGE_TABLE };
        let e0 = table[0x40];
        let e1 = table[KERNEL_PML4];
        assert!(!e1.is_unused());

        self.edit(|_| {
            table[0x40] = e0;
            table[KERNEL_PML4].set(e1.frame(), EF::VALID | EF::GLOBAL);
        });
    }
}

impl Drop for InactivePageTable0 {
    fn drop(&mut self) {
        info!("PageTable dropping: {:?}", self);
        Self::dealloc_frame(self.p2_frame.start_address().as_u32() as usize);
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
