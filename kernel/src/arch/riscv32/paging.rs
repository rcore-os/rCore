use crate::consts::RECURSIVE_INDEX;
// Depends on kernel
use crate::memory::{active_table, alloc_frame, dealloc_frame};
use riscv::addr::*;
use riscv::asm::{sfence_vma, sfence_vma_all};
use riscv::paging::{Mapper, PageTable as RvPageTable, PageTableEntry, PageTableFlags as EF, RecursivePageTable};
use riscv::paging::{FrameAllocator, FrameDeallocator};
use riscv::register::satp;
use ucore_memory::paging::*;
use log::*;
#[cfg(target_arch = "riscv32")]
use crate::consts::KERNEL_P2_INDEX;
#[cfg(target_arch = "riscv64")]
use crate::consts::KERNEL_P4_INDEX;

pub struct ActivePageTable(RecursivePageTable<'static>, PageEntry);

/// PageTableEntry: the contents of this entry.
/// Page: this entry is the pte of page `Page`.
pub struct PageEntry(PageTableEntry, Page);

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

    /*
    * @param:
    *   addr: virtual address of which the mapped physical frame should be unmapped
    * @bridf:
    ^   unmap the virtual addresses' mapped physical frame
    */
    fn unmap(&mut self, addr: usize) {
        let page = Page::of_addr(VirtAddr::new(addr));
        let (_, flush) = self.0.unmap(page).unwrap();
        flush.flush();
    }

    /*
    * @param:
    *   addr: input virtual address
    * @brief:
    *   get the pageEntry of 'addr'
    * @retval:
    *   a mutable PageEntry reference of 'addr'
    */
    #[cfg(target_arch = "riscv32")]
    fn get_entry(&mut self, vaddr: usize) -> Option<&mut Entry> {
        let p2_table = unsafe { ROOT_PAGE_TABLE.as_mut().unwrap() };
        let page = Page::of_addr(VirtAddr::new(vaddr));
        if !p2_table[page.p2_index()].flags().contains(EF::VALID) {
            return None;
        }
        let entry = edit_entry_of(&page, |entry| *entry);
        self.1 = PageEntry(entry, page);
        Some(&mut self.1)
    }

    /*
    * @param:
    *   addr: input virtual address
    * @brief:
    *   get the pageEntry of 'addr'
    * @retval:
    *   a mutable PageEntry reference of 'addr'
    */
    #[cfg(target_arch = "riscv64")]
    fn get_entry(&mut self, vaddr: usize) -> Option<&mut Entry> {
        let vaddr = VirtAddr::new(vaddr);
        let page = Page::of_addr(vaddr);

        if ! self.0.is_mapped(
                vaddr.p4_index(), vaddr.p3_index(), vaddr.p2_index(), vaddr.p1_index()) {
            return None;
        }

        let entry = edit_entry_of(&page, |entry| *entry);
        self.1 = PageEntry(entry, page);
        Some(&mut self.1)
    }
}

impl PageTableExt for ActivePageTable {}

#[cfg(target_arch = "riscv32")]
fn edit_entry_of<T>(page: &Page, f: impl FnOnce(&mut PageTableEntry) -> T) -> T {
    let p2_table = unsafe { ROOT_PAGE_TABLE.as_mut().unwrap() };
    let p1_table = unsafe {
        &mut *(Page::from_page_table_indices(RECURSIVE_INDEX, page.p2_index()).
               start_address().as_usize() as *mut RvPageTable)
    };
    let p2_flags = p2_table[page.p2_index()].flags_mut();

    p2_flags.insert(EF::READABLE | EF::WRITABLE);
    let ret = f(&mut p1_table[page.p1_index()]);
    p2_flags.remove(EF::READABLE | EF::WRITABLE);

    ret
}

// TODO: better the gofy design
#[cfg(target_arch = "riscv64")]
fn edit_entry_of<T>(page: &Page, f: impl FnOnce(&mut PageTableEntry) -> T) -> T {
    let p4_table = unsafe { ROOT_PAGE_TABLE.as_mut().unwrap() };
    let p3_table = unsafe {
        &mut *(Page::from_page_table_indices(
                RECURSIVE_INDEX, RECURSIVE_INDEX, RECURSIVE_INDEX,
                page.p4_index()).start_address().as_usize() as *mut RvPageTable)
    };
    let p2_table = unsafe {
        &mut *(Page::from_page_table_indices(
                RECURSIVE_INDEX, RECURSIVE_INDEX, page.p4_index(),
                page.p3_index()).start_address().as_usize() as *mut RvPageTable)
    };
    let p1_table = unsafe {
        &mut *(Page::from_page_table_indices(
                RECURSIVE_INDEX, page.p4_index(), page.p3_index(),
                page.p2_index()).start_address().as_usize() as *mut RvPageTable)
    };
    let p4_flags = p4_table[page.p4_index()].flags_mut();
    let p3_flags = p3_table[page.p3_index()].flags_mut();
    let p2_flags = p2_table[page.p2_index()].flags_mut();

    p4_flags.insert(EF::READABLE | EF::WRITABLE)         ; sfence_vma_all();
        p3_flags.insert(EF::READABLE | EF::WRITABLE)     ; sfence_vma_all();
    p4_flags.remove(EF::READABLE | EF::WRITABLE)         ; sfence_vma_all();
            p2_flags.insert(EF::READABLE | EF::WRITABLE) ; sfence_vma_all();
    p4_flags.insert(EF::READABLE | EF::WRITABLE)         ; sfence_vma_all();
        p3_flags.remove(EF::READABLE | EF::WRITABLE)     ; sfence_vma_all();
    p4_flags.remove(EF::READABLE | EF::WRITABLE)         ; sfence_vma_all();
    let ret = f(&mut p1_table[page.p1_index()])          ;
    p4_flags.insert(EF::READABLE | EF::WRITABLE)         ; sfence_vma_all();
        p3_flags.insert(EF::READABLE | EF::WRITABLE)     ; sfence_vma_all();
    p4_flags.remove(EF::READABLE | EF::WRITABLE)         ; sfence_vma_all();
            p2_flags.remove(EF::READABLE | EF::WRITABLE) ; sfence_vma_all();
    p4_flags.insert(EF::READABLE | EF::WRITABLE)         ; sfence_vma_all();
        p3_flags.remove(EF::READABLE | EF::WRITABLE)     ; sfence_vma_all();
    p4_flags.remove(EF::READABLE | EF::WRITABLE)         ; sfence_vma_all();

    ret
}



// define the ROOT_PAGE_TABLE, and the virtual address of it?
#[cfg(target_arch = "riscv32")]
const ROOT_PAGE_TABLE: *mut RvPageTable =
    (((RECURSIVE_INDEX << 10) | (RECURSIVE_INDEX + 1)) << 12) as *mut RvPageTable;
#[cfg(target_arch = "riscv64")]
const ROOT_PAGE_TABLE: *mut RvPageTable =
    ((0xFFFF_0000_0000_0000) |
     (RECURSIVE_INDEX     << 12 << 9 << 9 << 9) |
     (RECURSIVE_INDEX     << 12 << 9 << 9) |
     (RECURSIVE_INDEX     << 12 << 9) |
     ((RECURSIVE_INDEX+1) << 12)) as *mut RvPageTable;

impl ActivePageTable {
    pub unsafe fn new() -> Self {
        ActivePageTable(
            RecursivePageTable::new(&mut *ROOT_PAGE_TABLE).unwrap(),
            ::core::mem::zeroed()
        )
    }
}

/// implementation for the Entry trait in /crate/memory/src/paging/mod.rs
impl Entry for PageEntry {
    fn update(&mut self) {
        edit_entry_of(&self.1, |entry| *entry = self.0);
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
        let e0 = table[0x40];
        let e1 = table[KERNEL_P2_INDEX];
        let e2 = table[KERNEL_P2_INDEX + 1];
        let e3 = table[KERNEL_P2_INDEX + 2];

        self.edit(|_| {
            table[0x40] = e0;
            table[KERNEL_P2_INDEX] = e1;
            table[KERNEL_P2_INDEX + 1] = e2;
            table[KERNEL_P2_INDEX + 2] = e3;
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
        satp.set_bits(60..64, satp::Mode::Sv48 as usize);  // Mode is Sv48
        satp
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
