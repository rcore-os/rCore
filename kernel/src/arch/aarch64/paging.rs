//! Page table implementations for aarch64.
// Depends on kernel
use consts::{KERNEL_PML4, RECURSIVE_INDEX};
use memory::{active_table, alloc_frame, alloc_stack, dealloc_frame};
use ucore_memory::memory_set::*;
use ucore_memory::PAGE_SIZE;
use ucore_memory::paging::*;
use aarch64::asm::{tlb_invalidate, tlb_invalidate_all, ttbr_el1_read, ttbr_el1_write};
use aarch64::{PhysAddr, VirtAddr};
use aarch64::paging::{Mapper, PageTable as Aarch64PageTable, PageTableEntry, PageTableFlags as EF, RecursivePageTable};
use aarch64::paging::{FrameAllocator, FrameDeallocator, Page, PhysFrame as Frame, Size4KiB, Size2MiB, Size1GiB};
use aarch64::paging::memory_attribute::*;

// need 3 page
pub fn setup_temp_page_table(frame_lvl4: Frame, frame_lvl3: Frame, frame_lvl2: Frame) {
    let p4 = unsafe { &mut *(frame_lvl4.start_address().as_u64() as *mut Aarch64PageTable) };
    let p3 = unsafe { &mut *(frame_lvl3.start_address().as_u64() as *mut Aarch64PageTable) };
    let p2 = unsafe { &mut *(frame_lvl2.start_address().as_u64() as *mut Aarch64PageTable) };
    p4.zero();
    p3.zero();
    p2.zero();

    let (start_addr, end_addr) = (0, 0x40000000);
    let block_flags = EF::VALID | EF::AF | EF::WRITE | EF::XN;
    for page in Page::<Size2MiB>::range_of(start_addr, end_addr) {
        let paddr = PhysAddr::new(page.start_address().as_u64());

        use arch::board::IO_REMAP_BASE;
        if paddr.as_u64() >= IO_REMAP_BASE as u64 {
            p2[page.p2_index()].set_block::<Size2MiB>(paddr, block_flags | EF::PXN, MairDevice::attr_value());
        } else {
            p2[page.p2_index()].set_block::<Size2MiB>(paddr, block_flags, MairNormal::attr_value());
        }
    }

    p3[0].set_frame(frame_lvl2, EF::default(), MairNormal::attr_value());
    p3[1].set_block::<Size1GiB>(PhysAddr::new(0x40000000), block_flags | EF::PXN, MairDevice::attr_value());

    p4[0].set_frame(frame_lvl3, EF::default(), MairNormal::attr_value());
    p4[RECURSIVE_INDEX].set_frame(frame_lvl4, EF::default(), MairNormal::attr_value());

    ttbr_el1_write(0, frame_lvl4);
    tlb_invalidate_all();
}

pub struct ActivePageTable(RecursivePageTable<'static>);

pub struct PageEntry(PageTableEntry);

impl PageTable for ActivePageTable {
    type Entry = PageEntry;

    fn map(&mut self, addr: usize, target: usize) -> &mut PageEntry {
        let flags = EF::default();
        let attr = MairNormal::attr_value();
        self.0.map_to(Page::of_addr(addr), Frame::of_addr(target), flags, attr, &mut FrameAllocatorForAarch64)
            .unwrap().flush();
        self.get_entry(addr)
    }

    fn unmap(&mut self, addr: usize) {
        let (frame, flush) = self.0.unmap(Page::of_addr(addr)).unwrap();
        flush.flush();
    }

    fn get_entry(&mut self, addr: usize) -> &mut PageEntry {
        let entry_addr = ((addr >> 9) & 0o777_777_777_7770) | (RECURSIVE_INDEX << 39);
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

const ROOT_PAGE_TABLE: *mut Aarch64PageTable =
    ((RECURSIVE_INDEX << 39) | (RECURSIVE_INDEX << 30) | (RECURSIVE_INDEX << 21) | (RECURSIVE_INDEX << 12)) as *mut Aarch64PageTable;

impl ActivePageTable {
    pub unsafe fn new() -> Self {
        ActivePageTable(RecursivePageTable::new(&mut *(ROOT_PAGE_TABLE as *mut _)).unwrap())
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
        let addr = VirtAddr::new_unchecked((self as *const _ as u64) << 9);
        tlb_invalidate(addr);
    }

    fn present(&self) -> bool { self.0.flags().contains(EF::VALID) }
    fn accessed(&self) -> bool { self.0.flags().contains(EF::AF) }
    fn writable(&self) -> bool { self.0.flags().contains(EF::WRITE) }
    fn dirty(&self) -> bool { self.hw_dirty() && self.sw_dirty() }

    fn clear_accessed(&mut self) { self.as_flags().remove(EF::AF); }
    fn clear_dirty(&mut self)
    {
        self.as_flags().remove(EF::DIRTY);
        self.as_flags().insert(EF::AP_RO);
    }
    fn set_writable(&mut self, value: bool)
    {
        self.as_flags().set(EF::AP_RO, !value);
        self.as_flags().set(EF::WRITE, value);
    }
    fn set_present(&mut self, value: bool) { self.as_flags().set(EF::VALID, value); }
    fn target(&self) -> usize { self.0.addr().as_u64() as usize }
    fn set_target(&mut self, target: usize) {
        self.0.modify_addr(PhysAddr::new(target as u64));
    }
    fn writable_shared(&self) -> bool { self.0.flags().contains(EF::WRITABLE_SHARED) }
    fn readonly_shared(&self) -> bool { self.0.flags().contains(EF::READONLY_SHARED) }
    fn set_shared(&mut self, writable: bool) {
        let flags = self.as_flags();
        flags.set(EF::WRITABLE_SHARED, writable);
        flags.set(EF::READONLY_SHARED, !writable);
    }
    fn clear_shared(&mut self) { self.as_flags().remove(EF::WRITABLE_SHARED | EF::READONLY_SHARED); }
    fn user(&self) -> bool { self.0.flags().contains(EF::AP_EL0) }
    fn swapped(&self) -> bool { self.0.flags().contains(EF::SWAPPED) }
    fn set_swapped(&mut self, value: bool) { self.as_flags().set(EF::SWAPPED, value); }
    fn set_user(&mut self, value: bool) {
        self.as_flags().set(EF::AP_EL0, value);
        self.as_flags().set(EF::nG, value); // set non-global to use ASID
    }
    fn execute(&self) -> bool {
        match self.user() {
            true => !self.0.flags().contains(EF::XN),
            false => !self.0.flags().contains(EF::PXN),
        }
    }
    fn set_execute(&mut self, value: bool) {
        match self.user() {
            true => self.as_flags().set(EF::XN, !value),
            false => self.as_flags().set(EF::PXN, !value),
        }
    }
    fn mmio(&self) -> bool { self.0.attr().value == MairDevice::attr_value().value }
    fn set_mmio(&mut self, value: bool) { self.0.modify_attr(MairDevice::attr_value()); }
}

impl PageEntry {
    fn read_only(&self) -> bool { self.0.flags().contains(EF::AP_RO) }
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
        // When the new InactivePageTable is created for the user MemorySet, it's use ttbr1 as the
        // TTBR. And the kernel TTBR ttbr0 will never changed, so we needn't call map_kernel()
        Self::new_bare()
    }

    fn new_bare() -> Self {
        let frame = Self::alloc_frame().map(|target| Frame::of_addr(target))
            .expect("failed to allocate frame");
        active_table().with_temporary_map(&frame, |_, table: &mut Aarch64PageTable| {
            table.zero();
            // set up recursive mapping for the table
            table[RECURSIVE_INDEX].set_frame(frame.clone(), EF::default(), MairNormal::attr_value());
        });
        InactivePageTable0 { p4_frame: frame }
    }

    fn edit(&mut self, f: impl FnOnce(&mut Self::Active)) {
        active_table().with_temporary_map(&ttbr_el1_read(0), |active_table, p4_table: &mut Aarch64PageTable| {
            let backup = p4_table[RECURSIVE_INDEX].clone();

            // overwrite recursive mapping
            p4_table[RECURSIVE_INDEX].set_frame(self.p4_frame.clone(), EF::default(), MairNormal::attr_value());
            tlb_invalidate_all();

            // execute f in the new context
            f(active_table);

            // restore recursive mapping to original p4 table
            p4_table[RECURSIVE_INDEX] = backup;
            tlb_invalidate_all();
        });
    }

    unsafe fn activate(&self) {
        let old_frame = ttbr_el1_read(0);
        let new_frame = self.p4_frame.clone();
        debug!("switch TTBR0 {:?} -> {:?}", old_frame, new_frame);
        if old_frame != new_frame {
            ttbr_el1_write(0, new_frame);
            tlb_invalidate_all();
        }
    }

    unsafe fn with(&self, f: impl FnOnce()) {
        // Just need to switch the user TTBR
        let old_frame = ttbr_el1_read(1);
        let new_frame = self.p4_frame.clone();
        debug!("switch TTBR1 {:?} -> {:?}", old_frame, new_frame);
        if old_frame != new_frame {
            ttbr_el1_write(1, new_frame);
            tlb_invalidate_all();
        }
        f();
        debug!("switch TTBR1 {:?} -> {:?}", new_frame, old_frame);
        if old_frame != new_frame {
            ttbr_el1_write(1, old_frame);
            tlb_invalidate_all();
        }
    }

    fn token(&self) -> usize {
        self.p4_frame.start_address().as_u64() as usize // as TTBRx_EL1
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
        let e0 = table[KERNEL_PML4].clone();
        assert!(!e0.is_unused());

        self.edit(|_| {
            table[KERNEL_PML4].set_frame(Frame::containing_address(e0.addr()), EF::default(), MairNormal::attr_value());
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
