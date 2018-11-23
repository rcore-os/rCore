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
use aarch64::paging::{FrameAllocator, FrameDeallocator, Page, PageRange, PhysFrame as Frame, Size4KiB, Size2MiB};

register_bitfields! {u64,
    // AArch64 Reference Manual page 2150
    STAGE1_DESCRIPTOR [
        /// Execute-never
        XN       OFFSET(54) NUMBITS(1) [
            False = 0,
            True = 1
        ],

        /// Various address fields, depending on use case
        LVL4_OUTPUT_ADDR_4KiB    OFFSET(39) NUMBITS(9) [], // [47:39]
        LVL3_OUTPUT_ADDR_4KiB    OFFSET(30) NUMBITS(18) [], // [47:30]
        LVL2_OUTPUT_ADDR_4KiB    OFFSET(21) NUMBITS(27) [], // [47:21]
        NEXT_LVL_TABLE_ADDR_4KiB OFFSET(12) NUMBITS(36) [], // [47:12]

        /// Access flag
        AF       OFFSET(10) NUMBITS(1) [
            False = 0,
            True = 1
        ],

        /// Shareability field
        SH       OFFSET(8) NUMBITS(2) [
            OuterShareable = 0b10,
            InnerShareable = 0b11
        ],

        /// Access Permissions
        AP       OFFSET(6) NUMBITS(2) [
            RW_EL1 = 0b00,
            RW_EL1_EL0 = 0b01,
            RO_EL1 = 0b10,
            RO_EL1_EL0 = 0b11
        ],

        /// Memory attributes index into the MAIR_EL1 register
        AttrIndx OFFSET(2) NUMBITS(3) [],

        TYPE     OFFSET(1) NUMBITS(1) [
            Block = 0,
            Table = 1
        ],

        VALID    OFFSET(0) NUMBITS(1) [
            False = 0,
            True = 1
        ]
    ]
}

mod mair {
    pub const NORMAL: u64 = 0;
    pub const DEVICE: u64 = 1;
}

// need 3 page
pub fn setup_page_table(frame_lvl4: Frame, frame_lvl3: Frame, frame_lvl2: Frame) {
    let p4 = unsafe { &mut *(frame_lvl4.start_address().as_u64() as *mut Aarch64PageTable) };
    let p3 = unsafe { &mut *(frame_lvl3.start_address().as_u64() as *mut Aarch64PageTable) };
    let p2 = unsafe { &mut *(frame_lvl2.start_address().as_u64() as *mut Aarch64PageTable) };
    p4.zero();
    p3.zero();
    p2.zero();

    // Fill the rest of the LVL2 (2MiB) entries as block
    // descriptors. Differentiate between normal and device mem.
    const MMIO_BASE: u64 = 0x3F000000;
    let mmio_base: u64 = MMIO_BASE >> 21;
    let mut common = STAGE1_DESCRIPTOR::VALID::True
        + STAGE1_DESCRIPTOR::TYPE::Block
        + STAGE1_DESCRIPTOR::AP::RW_EL1
        + STAGE1_DESCRIPTOR::AF::True;
        // + STAGE1_DESCRIPTOR::XN::True;

    for i in 0..512 {
        let j: u64 = i as u64;

        let mem_attr = if j >= mmio_base {
            STAGE1_DESCRIPTOR::SH::OuterShareable + STAGE1_DESCRIPTOR::AttrIndx.val(mair::DEVICE)
        } else {
            STAGE1_DESCRIPTOR::SH::InnerShareable + STAGE1_DESCRIPTOR::AttrIndx.val(mair::NORMAL)
        };

        p2[i].entry = (common + mem_attr + STAGE1_DESCRIPTOR::LVL2_OUTPUT_ADDR_4KiB.val(j)).value;
    }

    common = common + STAGE1_DESCRIPTOR::SH::InnerShareable + STAGE1_DESCRIPTOR::AttrIndx.val(mair::NORMAL);

    p3[0].entry = (common + STAGE1_DESCRIPTOR::TYPE::Table + STAGE1_DESCRIPTOR::NEXT_LVL_TABLE_ADDR_4KiB.val(frame_lvl2.start_address().as_u64() >> 12)).value;
    p3[1].entry = (common + STAGE1_DESCRIPTOR::LVL3_OUTPUT_ADDR_4KiB.val(1)).value;
    p4[0].entry = (common + STAGE1_DESCRIPTOR::TYPE::Table + STAGE1_DESCRIPTOR::NEXT_LVL_TABLE_ADDR_4KiB.val(frame_lvl3.start_address().as_u64() >> 12)).value;
    p4[RECURSIVE_INDEX].entry = (common + STAGE1_DESCRIPTOR::TYPE::Table + STAGE1_DESCRIPTOR::NEXT_LVL_TABLE_ADDR_4KiB.val(frame_lvl4.start_address().as_u64() >> 12)).value;

    // warn!("p2");
    // for i in 0..512 {
    //     if p2[i].flags().bits() != 0 {
    //         info!("{:x?} {:x?} {:x?}",i, &p2[i] as *const _ as usize, p2[i]);
    //     }
    // }
    // warn!("p3");
    // for i in 0..512 {
    //     if p3[i].flags().bits() != 0 {
    //         info!("{:x?} {:x?} {:x?}",i, &p3[i] as *const _ as usize, p3[i]);
    //     }
    // }
    // warn!("p4");
    // for i in 0..512 {
    //     if p4[i].flags().bits() != 0 {
    //         info!("{:x?} {:x?} {:x?}",i, &p4[i] as *const _ as usize, p4[i]);
    //     }
    // }

    ttbr_el1_write(0, frame_lvl4);
    tlb_invalidate_all();
}

/// map the range [start, end) as device memory, insert to the MemorySet
pub fn remap_device_2mib(ms: &mut MemorySet<InactivePageTable0>, start_addr: usize, end_addr: usize) {
    ms.edit(|active_table| {
        let common = STAGE1_DESCRIPTOR::VALID::True
            + STAGE1_DESCRIPTOR::TYPE::Block
            + STAGE1_DESCRIPTOR::AP::RW_EL1
            + STAGE1_DESCRIPTOR::AF::True
            + STAGE1_DESCRIPTOR::XN::True;

        let mem_attr = STAGE1_DESCRIPTOR::SH::OuterShareable + STAGE1_DESCRIPTOR::AttrIndx.val(mair::DEVICE);

        type Page2MiB = Page<Size2MiB>;
        for page in Page2MiB::range_of(start_addr, end_addr) {
            let p2 = unsafe { &mut *active_table.0.p2_ptr(page) };
            p2[page.p2_index()].entry = (common + mem_attr + STAGE1_DESCRIPTOR::LVL2_OUTPUT_ADDR_4KiB.val(page.start_address().as_u64() >> 21)).value;
        }

        // let p2 = unsafe { &mut *(0o777_777_000_000_0000 as *mut Aarch64PageTable) };
        // for i in 0..512 {
        //     if p2[i].flags().bits() != 0 {
        //         info!("{:x?} {:x?} {:x?}",i, &p2[i] as *const _ as usize, p2[i]);
        //     }
        // }

        // let p2 = unsafe { &mut *(0o777_777_000_001_0000 as *mut Aarch64PageTable) };
        // for i in 0..512 {
        //     if p2[i].flags().bits() != 0 {
        //         info!("{:x?} {:x?} {:x?}",i, &p2[i] as *const _ as usize, p2[i]);
        //     }
        // }
    });
}

pub struct ActivePageTable(RecursivePageTable<'static>);

pub struct PageEntry(PageTableEntry);

impl PageTable for ActivePageTable {
    type Entry = PageEntry;

    fn map(&mut self, addr: usize, target: usize) -> &mut PageEntry {
        let flags = EF::PRESENT | EF::WRITE | EF::ACCESSED | EF::UXN | EF::PAGE_BIT;
        self.0.map_to(Page::of_addr(addr), Frame::of_addr(target), flags, &mut FrameAllocatorForAarch64)
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
        self.as_flags().set(EF::NONE_GLOBAL, value); // set non-global to use ASID
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
            table[RECURSIVE_INDEX].set_frame(frame.clone(), EF::PRESENT | EF::WRITE | EF::ACCESSED | EF::PAGE_BIT);
        });
        InactivePageTable0 { p4_frame: frame }
    }

    fn edit(&mut self, f: impl FnOnce(&mut Self::Active)) {
        active_table().with_temporary_map(&ttbr_el1_read(0), |active_table, p4_table: &mut Aarch64PageTable| {
            let backup = p4_table[RECURSIVE_INDEX].clone();

            // overwrite recursive mapping
            p4_table[RECURSIVE_INDEX].set_frame(self.p4_frame.clone(), EF::PRESENT | EF::WRITE | EF::ACCESSED | EF::PAGE_BIT);
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
            table[KERNEL_PML4].set_addr(e0.addr(), e0.flags() & EF::GLOBAL);
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
