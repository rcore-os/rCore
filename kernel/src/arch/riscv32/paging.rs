use crate::consts::RECURSIVE_INDEX;
// Depends on kernel
use crate::memory::{active_table, alloc_frame, dealloc_frame};
use riscv::addr::*;
use riscv::asm::{sfence_vma, sfence_vma_all};
use riscv::paging::{Mapper, PageTable as RvPageTable, PageTableEntry, PageTableFlags as EF, RecursivePageTable, PageTableType};
use riscv::paging::{FrameAllocator, FrameDeallocator};
use riscv::register::satp;
use rcore_memory::paging::*;
use log::*;
#[cfg(target_arch = "riscv32")]
use crate::consts::KERNEL_P2_INDEX;
#[cfg(target_arch = "riscv64")]
use crate::consts::KERNEL_P4_INDEX;

pub struct ActivePageTable(RecursivePageTable<'static>, PageEntry);

/// PageTableEntry: the contents of this entry.
/// Page: this entry is the pte of page `Page`.
pub struct PageEntry(&'static mut PageTableEntry, Page);

impl PageTable for ActivePageTable {

    fn map(&mut self, addr: usize, target: usize) -> &mut Entry {
        // use riscv::paging:Mapper::map_to,
        // map the 4K `page` to the 4K `frame` with `flags`
        let flags = EF::VALID | EF::READABLE | EF::WRITABLE;
        let page = Page::of_addr(VirtAddr::new(addr));
        let frame = Frame::of_addr(PhysAddr::new(target));
        // map the page to the frame using FrameAllocatorForRiscv
        // we may need frame allocator to alloc frame for new page table(first/second)
        self.0.map_to(page, frame, flags, &mut FrameAllocatorForRiscv).unwrap().flush();
        self.get_entry(addr).expect("fail to get entry")
    }

    fn unmap(&mut self, addr: usize) {
        let page = Page::of_addr(VirtAddr::new(addr));
        let (_, flush) = self.0.unmap(page).unwrap();
        flush.flush();
    }

    fn get_entry(&mut self, vaddr: usize) -> Option<&mut Entry> {
        let page = Page::of_addr(VirtAddr::new(vaddr));
        if let Ok(e) = self.0.ref_entry(page.clone()) {
            let e = unsafe { &mut *(e as *mut PageTableEntry) };
            self.1 = PageEntry(e, page);
            Some(&mut self.1 as &mut Entry)
        } else {
            None
        }
    }
}

impl PageTableExt for ActivePageTable {}

// define the ROOT_PAGE_TABLE, and the virtual address of it?
#[cfg(target_arch = "riscv32")]
const ROOT_PAGE_TABLE: *mut RvPageTable =
    ((RECURSIVE_INDEX     << 12 << 10) |
     ((RECURSIVE_INDEX+1) << 12)) as *mut RvPageTable;
#[cfg(all(target_arch = "riscv64", feature = "sv39"))]
const ROOT_PAGE_TABLE: *mut RvPageTable =
    ((0xFFFF_0000_0000_0000) |
     (0o777               << 12 << 9 << 9 << 9) |
     (RECURSIVE_INDEX     << 12 << 9 << 9) |
     (RECURSIVE_INDEX     << 12 << 9) |
     ((RECURSIVE_INDEX+1) << 12)) as *mut RvPageTable;
#[cfg(all(target_arch = "riscv64", not(feature = "sv39")))]
const ROOT_PAGE_TABLE: *mut RvPageTable =
    ((0xFFFF_0000_0000_0000) |
     (RECURSIVE_INDEX     << 12 << 9 << 9 << 9) |
     (RECURSIVE_INDEX     << 12 << 9 << 9) |
     (RECURSIVE_INDEX     << 12 << 9) |
     ((RECURSIVE_INDEX+1) << 12)) as *mut RvPageTable;

impl ActivePageTable {
    #[cfg(target_arch = "riscv32")]
    pub unsafe fn new() -> Self {
        ActivePageTable(
            RecursivePageTable::new(&mut *ROOT_PAGE_TABLE).unwrap(),
            ::core::mem::uninitialized()
        )
    }
    #[cfg(target_arch = "riscv64")]
    pub unsafe fn new() -> Self {
        #[cfg(feature = "sv39")]
        let type_ = PageTableType::Sv39;
        #[cfg(not(feature = "sv39"))]
        let type_ = PageTableType::Sv48;
        ActivePageTable(
            RecursivePageTable::new(&mut *ROOT_PAGE_TABLE, type_).unwrap(),
            ::core::mem::uninitialized()
        )
    }
}

/// implementation for the Entry trait in /crate/memory/src/paging/mod.rs
impl Entry for PageEntry {
    fn update(&mut self) {
        sfence_vma(0, self.1.start_address());
    }
    fn accessed(&self) -> bool { self.0.flags().contains(EF::ACCESSED) }
    fn dirty(&self) -> bool { self.0.flags().contains(EF::DIRTY) }
    fn writable(&self) -> bool { self.0.flags().contains(EF::WRITABLE) }
    fn present(&self) -> bool { self.0.flags().contains(EF::VALID | EF::READABLE) }
    fn clear_accessed(&mut self) { self.0.flags_mut().remove(EF::ACCESSED); }
    fn clear_dirty(&mut self) { self.0.flags_mut().remove(EF::DIRTY); }
    fn set_writable(&mut self, value: bool) { self.0.flags_mut().set(EF::WRITABLE, value); }
    fn set_present(&mut self, value: bool) { self.0.flags_mut().set(EF::VALID | EF::READABLE, value); }
    fn target(&self) -> usize { self.0.addr().as_usize() }
    fn set_target(&mut self, target: usize) {
        let flags = self.0.flags();
        let frame = Frame::of_addr(PhysAddr::new(target));
        self.0.set(frame, flags);
    }
    fn writable_shared(&self) -> bool { self.0.flags().contains(EF::RESERVED1) }
    fn readonly_shared(&self) -> bool { self.0.flags().contains(EF::RESERVED2) }
    fn set_shared(&mut self, writable: bool) {
        let flags = self.0.flags_mut();
        flags.set(EF::RESERVED1, writable);
        flags.set(EF::RESERVED2, !writable);
    }
    fn clear_shared(&mut self) { self.0.flags_mut().remove(EF::RESERVED1 | EF::RESERVED2); }
    fn swapped(&self) -> bool { self.0.flags().contains(EF::RESERVED1) }
    fn set_swapped(&mut self, value: bool) { self.0.flags_mut().set(EF::RESERVED1, value); }
    fn user(&self) -> bool { self.0.flags().contains(EF::USER) }
    fn set_user(&mut self, value: bool) { self.0.flags_mut().set(EF::USER, value); }
    fn execute(&self) -> bool { self.0.flags().contains(EF::EXECUTABLE) }
    fn set_execute(&mut self, value: bool) { self.0.flags_mut().set(EF::EXECUTABLE, value); }
    fn mmio(&self) -> u8 { 0 }
    fn set_mmio(&mut self, _value: u8) { }
}

#[derive(Debug)]
pub struct InactivePageTable0 {
    root_frame: Frame,
}

impl InactivePageTable for InactivePageTable0 {
    type Active = ActivePageTable;

    fn new_bare() -> Self {
        let target = alloc_frame().expect("failed to allocate frame");
        let frame = Frame::of_addr(PhysAddr::new(target));
        active_table().with_temporary_map(target, |_, table: &mut RvPageTable| {
            table.zero();
            table.set_recursive(RECURSIVE_INDEX, frame.clone());
        });
        InactivePageTable0 { root_frame: frame }
    }

    /*
    * @brief:
    *   map the kernel code memory address (p2 page table) in the new inactive page table according the current active page table
    */
    #[cfg(target_arch = "riscv32")]
    fn map_kernel(&mut self) {
        let table = unsafe { &mut *ROOT_PAGE_TABLE };
        extern {
            fn start();
            fn end();
        }
        let mut entrys: [PageTableEntry; 16] = unsafe { core::mem::uninitialized() };
        let entry_start = start as usize >> 22;
        let entry_end = (end as usize >> 22) + 1;
        let entry_count = entry_end - entry_start;
        for i in 0..entry_count {
            entrys[i] = table[entry_start + i];
        }

        self.edit(|_| {
            // NOTE: 'table' now refers to new page table
            for i in 0..entry_count {
                table[entry_start + i] = entrys[i];
            }
        });
    }

    #[cfg(target_arch = "riscv64")]
    fn map_kernel(&mut self) {
        let table = unsafe { &mut *ROOT_PAGE_TABLE };
        let e1 = table[KERNEL_P4_INDEX];
        assert!(!e1.is_unused());

        self.edit(|_| {
            table[KERNEL_P4_INDEX] = e1;
        });
    }

    #[cfg(target_arch = "riscv32")]
    fn token(&self) -> usize {
        self.root_frame.number() | (1 << 31) // as satp
    }
    #[cfg(target_arch = "riscv64")]
    fn token(&self) -> usize {
        use bit_field::BitField;
        let mut satp = self.root_frame.number();
        satp.set_bits(44..60, 0);  // AS is 0
        #[cfg(feature = "sv39")]
        satp.set_bits(60..64, satp::Mode::Sv39 as usize);
        #[cfg(not(feature = "sv39"))]
        satp.set_bits(60..64, satp::Mode::Sv48 as usize);
        satp
    }

    unsafe fn set_token(token: usize) {
        asm!("csrw satp, $0" :: "r"(token) :: "volatile");
    }

    fn active_token() -> usize {
        satp::read().bits()
    }

    fn flush_tlb() {
        sfence_vma_all();
    }

    /*
    * @param:
    *   f: a function to do something with the temporary modified activate page table
    * @brief:
    *   temporarily make current `active_table`'s recursive entry point to
    *    `this` inactive table, so we can modify this inactive page table.
    */
    fn edit<T>(&mut self, f: impl FnOnce(&mut Self::Active) -> T) -> T {
        let target = satp::read().frame().start_address().as_usize();
        active_table().with_temporary_map(target, |active_table, root_table: &mut RvPageTable| {
            let backup = root_table[RECURSIVE_INDEX].clone();

            // overwrite recursive mapping
            root_table[RECURSIVE_INDEX].set(self.root_frame.clone(), EF::VALID);
            sfence_vma_all();

            // execute f in the new context
            let ret = f(active_table);

            // restore recursive mapping to original p2 table
            root_table[RECURSIVE_INDEX] = backup;
            sfence_vma_all();

            ret
        })
    }
}

impl Drop for InactivePageTable0 {
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
