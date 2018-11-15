//! Page table implementations for aarch64.
use bit_allocator::{BitAlloc};
// Depends on kernel
use memory::{active_table, alloc_frame, alloc_stack, dealloc_frame};
use ucore_memory::memory_set::*;
use ucore_memory::PAGE_SIZE;
use ucore_memory::paging::*;
use aarch64::asm::{tlb_invalidate, ttbr0_el1_read, ttbr0_el1_write};
use aarch64::{PhysAddr, VirtAddr};
use aarch64::paging::{Mapper, PageTable as Aarch64PageTable, PageTableEntry, PageTableFlags as EF, RecursivePageTable};
use aarch64::paging::{FrameAllocator, FrameDeallocator, Page, PageRange, PhysFrame as Frame, Size4KiB};
use aarch64::{regs::*};

pub trait PageExt {
    fn of_addr(address: usize) -> Self;
    fn range_of(begin: usize, end: usize) -> PageRange;
}

impl PageExt for Page {
    fn of_addr(address: usize) -> Self {
        Page::containing_address(VirtAddr::new(address as u64))
    }
    fn range_of(begin: usize, end: usize) -> PageRange {
        Page::range(Page::of_addr(begin), Page::of_addr(end - 1) + 1)
    }
}

pub trait FrameExt {
    fn of_addr(address: usize) -> Self;
}

impl FrameExt for Frame {
    fn of_addr(address: usize) -> Self {
        Frame::containing_address(PhysAddr::new(address as u64))
    }
}

pub struct ActivePageTable(RecursivePageTable<'static>);

pub struct PageEntry(PageTableEntry);

impl PageTable for ActivePageTable {
    type Entry = PageEntry;

    fn map(&mut self, addr: usize, target: usize) -> &mut PageEntry {
        let flags = EF::PRESENT | EF::WRITE | EF::UXN;
        self.0.map_to(Page::of_addr(addr), Frame::of_addr(target), flags, &mut FrameAllocatorForAarch64)
            .unwrap().flush();
        self.get_entry(addr)
    }

    fn unmap(&mut self, addr: usize) {
        let (frame, flush) = self.0.unmap(Page::of_addr(addr)).unwrap();
        flush.flush();
    }

    fn get_entry(&mut self, addr: usize) -> &mut PageEntry {
        let entry_addr = ((addr >> 9) & 0o777_777_777_7770) | 0xffffff80_00000000;
        unsafe { &mut *(entry_addr as *mut PageEntry) }
    }

    fn get_page_slice_mut<'a, 'b>(&'a mut self, addr: usize) -> &'b mut [u8] {
        use core::slice;
        unsafe { slice::from_raw_parts_mut((addr & !0xfffusize) as *mut u8, PAGE_SIZE) }
    }

    fn read(&mut self, addr: usize) -> u8 {
        unsafe { *(addr as *const u8) }
    }

    fn write(&mut self, addr: usize, data: u8) {
        unsafe { *(addr as *mut u8) = data; }
    }
}

impl ActivePageTable {
    pub unsafe fn new() -> Self {
        ActivePageTable(RecursivePageTable::new(&mut *(0xffffffff_fffff000 as *mut _)).unwrap())
    }
    fn with_temporary_map(&mut self, frame: &Frame, f: impl FnOnce(&mut ActivePageTable, &mut Aarch64PageTable)) {
        // Create a temporary page
        let page = Page::of_addr(0xcafebabe);
        assert!(self.0.translate_page(page).is_none(), "temporary page is already mapped");
        // Map it to table
        self.map(page.start_address().as_u64() as usize, frame.start_address().as_u64() as usize);
        // Call f
        let table = unsafe { &mut *page.start_address().as_mut_ptr() };
        f(self, table);
        // Unmap the page
        self.unmap(0xcafebabe);
    }
}

impl Entry for PageEntry {
    fn update(&mut self) {
        tlb_invalidate();
    }

    fn present(&self) -> bool { self.0.flags().contains(EF::PRESENT) }
    fn accessed(&self) -> bool { self.0.flags().contains(EF::ACCESSED) }
    fn writable(&self) -> bool { self.0.flags().contains(EF::WRITE) }
    fn dirty(&self) -> bool { self.hw_dirty() && self.sw_dirty() }

    fn clear_accessed(&mut self) { self.as_flags().remove(EF::ACCESSED); }
    fn clear_dirty(&mut self)
    {
        self.as_flags().remove(EF::DIRTY);
        self.as_flags().insert(EF::RDONLY);
    }
    fn set_writable(&mut self, value: bool)
    {
        self.as_flags().set(EF::RDONLY, !value);
        self.as_flags().set(EF::WRITE, value);
    }
    fn set_present(&mut self, value: bool) { self.as_flags().set(EF::PRESENT, value); }
    fn target(&self) -> usize { self.0.addr().as_u64() as usize }
    fn set_target(&mut self, target: usize) {
        let flags = self.0.flags();
        self.0.set_addr(PhysAddr::new(target as u64), flags);
    }
    fn writable_shared(&self) -> bool { self.0.flags().contains(EF::BIT_9) }
    fn readonly_shared(&self) -> bool { self.0.flags().contains(EF::BIT_9) }
    fn set_shared(&mut self, writable: bool) {
        let flags = self.as_flags();
        flags.set(EF::BIT_8, writable);
        flags.set(EF::BIT_9, writable);
    }
    fn clear_shared(&mut self) { self.as_flags().remove(EF::BIT_8 | EF::BIT_9); }
    fn user(&self) -> bool { self.0.flags().contains(EF::USER_ACCESSIBLE) }
    fn swapped(&self) -> bool { self.0.flags().contains(EF::SWAPPED) }
    fn set_swapped(&mut self, value: bool) { self.as_flags().set(EF::SWAPPED, value); }
    fn set_user(&mut self, value: bool) {
        self.as_flags().set(EF::USER_ACCESSIBLE, value);
        if value {
            let mut addr = self as *const _ as usize;
            for _ in 0..3 {
                // Upper level entry
                addr = ((addr >> 9) & 0o777_777_777_7770) | 0xffffff80_00000000;
                // set USER_ACCESSIBLE
                unsafe { (*(addr as *mut EF)).insert(EF::USER_ACCESSIBLE) };
            }
        }
    }
    fn execute(&self) -> bool { !self.0.flags().contains(EF::UXN) }
    fn set_execute(&mut self, value: bool) { self.as_flags().set(EF::UXN, !value); }
}

impl PageEntry {
    fn read_only(&self) -> bool { self.0.flags().contains(EF::RDONLY) }
    fn hw_dirty(&self) -> bool { self.writable() && !self.read_only() }
    fn sw_dirty(&self) -> bool { self.0.flags().contains(EF::DIRTY) }
    fn as_flags(&mut self) -> &mut EF {
        unsafe { &mut *(self as *mut _ as *mut EF) }
    }
}

#[derive(Debug)]
pub struct InactivePageTable0 {
    p4_frame: Frame,
}

impl InactivePageTable for InactivePageTable0 {
    type Active = ActivePageTable;

    fn new() -> Self {
        let mut pt = Self::new_bare();
        pt.map_kernel();
        pt
    }

    fn new_bare() -> Self {
        let frame = Self::alloc_frame().map(|target| Frame::of_addr(target))
            .expect("failed to allocate frame");
        active_table().with_temporary_map(&frame, |_, table: &mut Aarch64PageTable| {
            table.zero();
            // set up recursive mapping for the table
            table[511].set_frame(frame.clone(), EF::PRESENT | EF::WRITE);
        });
        InactivePageTable0 { p4_frame: frame }
    }

    fn edit(&mut self, f: impl FnOnce(&mut Self::Active)) {
        active_table().with_temporary_map(&ttbr0_el1_read().0, |active_table, p4_table: &mut Aarch64PageTable| {
            let backup = p4_table[0o777].clone();

            // overwrite recursive mapping
            p4_table[0o777].set_frame(self.p4_frame.clone(), EF::PRESENT | EF::WRITE);
            tlb_invalidate();

            // execute f in the new context
            f(active_table);

            // restore recursive mapping to original p4 table
            p4_table[0o777] = backup;
            tlb_invalidate();
        });
    }

    unsafe fn activate(&self) {
        let old_frame = ttbr0_el1_read().0;
        let new_frame = self.p4_frame.clone();
        debug!("switch table {:?} -> {:?}", old_frame, new_frame);
        if old_frame != new_frame {
            ttbr0_el1_write(new_frame);
        }
    }

    unsafe fn with(&self, f: impl FnOnce()) {
        let old_frame = ttbr0_el1_read().0;
        let new_frame = self.p4_frame.clone();
        debug!("switch table {:?} -> {:?}", old_frame, new_frame);
        if old_frame != new_frame {
            ttbr0_el1_write(new_frame);
        }
        f();
        debug!("switch table {:?} -> {:?}", new_frame, old_frame);
        if old_frame != new_frame {
            ttbr0_el1_write(old_frame);
        }
    }

    fn token(&self) -> usize {
        self.p4_frame.start_address().as_u64() as usize // as CR3
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
        let mut table = unsafe { &mut *(0xffffffff_fffff000 as *mut Aarch64PageTable) };
        // Kernel at 0xffff_ff00_0000_0000
        // Kernel stack at 0x0000_57ac_0000_0000 (defined in bootloader crate)
        let e0 = table[0].clone();
        self.edit(|_| {
            table[0].set_addr(e0.addr(), e0.flags() & EF::GLOBAL);
            //table[175].set_addr(estack.addr(), estack.flags() & EF::GLOBAL);
        });
    }
}

impl Drop for InactivePageTable0 {
    fn drop(&mut self) {
        info!("PageTable dropping: {:?}", self);
        Self::dealloc_frame(self.p4_frame.start_address().as_u64() as usize);
    }
}

struct FrameAllocatorForAarch64;

impl FrameAllocator<Size4KiB> for FrameAllocatorForAarch64 {
    fn alloc(&mut self) -> Option<Frame> {
        alloc_frame().map(|addr| Frame::of_addr(addr))
    }
}

impl FrameDeallocator<Size4KiB> for FrameAllocatorForAarch64 {
    fn dealloc(&mut self, frame: Frame) {
        dealloc_frame(frame.start_address().as_u64() as usize);
    }
}