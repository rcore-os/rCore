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

pub struct ActivePageTable(RecursivePageTable<'static>);

pub struct PageEntry(PageTableEntry);

impl PageTable for ActivePageTable {
    type Entry = PageEntry;

    /*
    * @param:
    *   addr: the virtual addr to be matched
    *   target: the physical addr to be matched with addr
    * @brief:
    *   map the virtual address 'addr' to the physical address 'target' in pagetable.
    * @retval:
    *   the matched PageEntry
    */
    fn map(&mut self, addr: usize, target: usize) -> &mut PageEntry {
        // use riscv::paging:Mapper::map_to,
        // map the 4K `page` to the 4K `frame` with `flags`
        let flags = EF::VALID | EF::READABLE | EF::WRITABLE;
        let page = Page::of_addr(VirtAddr::new(addr));
        let frame = Frame::of_addr(PhysAddr::new(target));
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
    *   addr: input virtual address
    * @brief:
    *   get the pageEntry of 'addr'
    * @retval:
    *   a mutable PageEntry reference of 'addr'
    */
    #[cfg(target_arch = "riscv32")]
    fn get_entry(&mut self, vaddr: usize) -> Option<&mut PageEntry> {
        let vaddr = VirtAddr::new(vaddr);
        if unsafe { !(*ROOT_PAGE_TABLE)[vaddr.p2_index()].flags().contains(EF::VALID) } {
            return None;
        }
        let page = Page::of_addr(vaddr);
        self.0.translate_page(page);
        let entry_va = VirtAddr::from_page_table_indices(RECURSIVE_INDEX,
                                                         vaddr.p2_index(),
                                                         vaddr.p1_index() << 2);
        unsafe { Some(&mut *(entry_va.as_usize() as *mut PageEntry)) }
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
    fn get_entry(&mut self, vaddr: usize) -> Option<&mut PageEntry> {
        let vaddr = VirtAddr::new(vaddr);
        let root_table: &RvPageTable = unsafe { &*ROOT_PAGE_TABLE };

        let p3_table = if ! root_table[vaddr.p4_index()].flags().contains(EF::VALID) {
            return None
        } else {
            let p3_table = unsafe { &mut *(Page::from_page_table_indices(
                self.0.recursive_index,
                self.0.recursive_index,
                self.0.recursive_index,
                vaddr.p4_index()).start_address().as_usize() as *mut RvPageTable) };
            p3_table
        };

        let p2_table = if ! p3_table[vaddr.p3_index()].flags().contains(EF::VALID) {
            return None
        } else {
            let p2_table = unsafe { &mut *(Page::from_page_table_indices(
                self.0.recursive_index,
                self.0.recursive_index,
                vaddr.p4_index(),
                vaddr.p3_index()).start_address().as_usize() as *mut RvPageTable) };
            p2_table
        };

        let p1_table = if ! p2_table[vaddr.p2_index()].flags().contains(EF::VALID) {
            return None
        } else {
            let p1_table = unsafe { &mut *(Page::from_page_table_indices(
                self.0.recursive_index,
                vaddr.p4_index(),
                vaddr.p3_index(),
                vaddr.p2_index()).start_address().as_usize() as *mut RvPageTable) };
            p1_table
        };

        let page = Page::of_addr(vaddr);
        // ???
        self.0.translate_page(page);
        let entry_va = VirtAddr::from_page_table_indices(RECURSIVE_INDEX,
                                                         vaddr.p4_index(), 
                                                         vaddr.p3_index(), 
                                                         vaddr.p2_index(),
                                                         vaddr.p1_index() << 3);
        unsafe { Some(&mut *(entry_va.as_usize() as *mut PageEntry)) }
    }


    /*
    * @param:
    *   addr:the input (virutal) address
    * @brief:
    *   get the addr's memory page slice
    * @retval:
    *   a mutable reference slice of 'addr' 's page
    */
    fn get_page_slice_mut<'a, 'b>(&'a mut self, addr: usize) -> &'b mut [u8] {
        use core::slice;
        unsafe { slice::from_raw_parts_mut((addr & !(PAGE_SIZE - 1)) as *mut u8, PAGE_SIZE) }
    }

    /*
    * @param:
    *   addr: virtual address
    * @brief:
    *   get the address's content
    * @retval:
    *   the content(u8) of 'addr'
    */
    fn read(&mut self, addr: usize) -> u8 {
        unsafe { *(addr as *const u8) }
    }

    /*
    * @param:
    *   addr: virtual address
    * @brief:
    *   write the address's content
    */
    fn write(&mut self, addr: usize, data: u8) {
        unsafe { *(addr as *mut u8) = data; }
    }
}

// define the ROOT_PAGE_TABLE, and the virtual address of it?
#[cfg(target_arch = "riscv32")]
const ROOT_PAGE_TABLE: *mut RvPageTable =
    (((RECURSIVE_INDEX << 10) | (RECURSIVE_INDEX + 1)) << 12) as *mut RvPageTable;
#[cfg(target_arch = "riscv64")]
const ROOT_PAGE_TABLE: *mut RvPageTable =
    ((RECURSIVE_INDEX    << 12 << 9 << 9 << 9) |
    (RECURSIVE_INDEX     << 12 << 9 << 9) |
    (RECURSIVE_INDEX     << 12 << 9) |
    ((RECURSIVE_INDEX+1) << 12)) as *mut RvPageTable;

impl ActivePageTable {
    pub unsafe fn new() -> Self {
        ActivePageTable(RecursivePageTable::new(&mut *ROOT_PAGE_TABLE).unwrap())
    }

    /*
    * @param:
    *   frame: the target physical frame which will be temporarily mapped
    *   f: the function you would like to apply for once
    * @brief:
    *   do something on the target physical frame?
    */
    #[cfg(target_arch = "riscv64")]
    fn with_temporary_map(&mut self, frame: &Frame, f: impl FnOnce(&mut ActivePageTable, &mut RvPageTable)) {
        // Create a temporary page
        let page = Page::of_addr(VirtAddr::new(0xffffffffcafebabe));
        assert!(self.0.translate_page(page).is_none(), "temporary page is already mapped");
        // Map it to table
        self.map(page.start_address().as_usize(), frame.start_address().as_usize());
        // Call f
        let table = unsafe { &mut *(page.start_address().as_usize() as *mut _) };
        f(self, table);
        // Unmap the page
        self.unmap(0xffffffffcafebabe);
    }

    #[cfg(target_arch = "riscv32")]
    fn with_temporary_map(&mut self, frame: &Frame, f: impl FnOnce(&mut ActivePageTable, &mut RvPageTable)) {
        // Create a temporary page
        let page = Page::of_addr(VirtAddr::new(0xcafebabe));
        assert!(self.0.translate_page(page).is_none(), "temporary page is already mapped");
        // Map it to table
        self.map(page.start_address().as_usize(), frame.start_address().as_usize());
        // Call f
        let table = unsafe { &mut *(page.start_address().as_usize() as *mut _) };
        f(self, table);
        // Unmap the page
        self.unmap(0xcafebabe);
    }
}

/// implementation for the Entry trait in /crate/memory/src/paging/mod.rs
impl Entry for PageEntry {
    #[cfg(target_arch = "riscv64")]
    fn update(&mut self) {
        let mut addr: usize = (self as *const _ as usize) << 9;
        if (addr & 0x7000_0000_0000 != 0) {
            addr |= 0xFFFF_0000_0000_0000;
        } else {
            addr &= 0x0000_FFFF_FFFF_FFFF;
        }
        sfence_vma(0, VirtAddr::new(addr));
    }
    #[cfg(target_arch = "riscv32")]
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
    fn target(&self) -> usize { self.0.addr().as_usize() }
    fn set_target(&mut self, target: usize) {
        let flags = self.0.flags();
        let frame = Frame::of_addr(PhysAddr::new(target));
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
    root_frame: Frame,
}

impl InactivePageTable for InactivePageTable0 {
    type Active = ActivePageTable;

    /*
    * @brief:
    *   get a new pagetable (for a new process or thread)
    * @retbal:
    *   the new pagetable
    */
    fn new() -> Self {
        let mut pt = Self::new_bare();
        pt.map_kernel();
        pt
    }

    /*
    * @brief:
    *   allocate a new frame and then self-mapping it and regard it as the inactivepagetale
    * retval:
    *   the inactive page table
    */
    fn new_bare() -> Self {
        let frame = Self::alloc_frame().map(|target| Frame::of_addr(PhysAddr::new(target)))
            .expect("failed to allocate frame");
        active_table().with_temporary_map(&frame, |_, table: &mut RvPageTable| {
            table.zero();
            table.set_recursive(RECURSIVE_INDEX, frame.clone());
        });
        InactivePageTable0 { root_frame: frame }
    }

    /*
    * @param:
    *   f: a function to do something with the temporary modified activate page table
    * @brief:
    *   temporarily make current `active_table`'s recursive entry point to 
    *    `this` inactive table, so we can modify this inactive page table.
    */
    fn edit(&mut self, f: impl FnOnce(&mut Self::Active)) {
        active_table().with_temporary_map(&satp::read().frame(), |active_table, root_table: &mut RvPageTable| {
            let backup = root_table[RECURSIVE_INDEX].clone();

            // overwrite recursive mapping
            root_table[RECURSIVE_INDEX].set(self.root_frame.clone(), EF::VALID);
            sfence_vma_all();

            // execute f in the new context
            f(active_table);

            // restore recursive mapping to original p2 table
            root_table[RECURSIVE_INDEX] = backup;
            sfence_vma_all();
        });
    }

    /*
    * @brief:
    *   active self as the current active page table
    */
    unsafe fn activate(&self) {
        let old_frame = satp::read().frame();
        let new_frame = self.root_frame.clone();
        debug!("switch table {:x?} -> {:x?}", old_frame, new_frame);
        if old_frame != new_frame {
            satp::set(SATP_MODE, 0, new_frame);
            sfence_vma_all();
        }
    }

    /*
    * @param:
    *   f: the function to run when temporarily activate self as current page table
    * @brief:
    *   Temporarily activate self and run the process, and return the return value of f
    * @retval:
    *   the return value of f
    */
    unsafe fn with<T>(&self, f: impl FnOnce() -> T) -> T {
        let old_frame = satp::read().frame();
        let new_frame = self.root_frame.clone();
        debug!("switch table {:x?} -> {:x?}", old_frame, new_frame);
        if old_frame != new_frame {
            satp::set(SATP_MODE, 0, new_frame);
            sfence_vma_all();
        }
        let target = f();
        debug!("switch table {:x?} -> {:x?}", new_frame, old_frame);
        if old_frame != new_frame {
            satp::set(SATP_MODE, 0, old_frame);
            sfence_vma_all();
        }
        target
    }

    /*
    * @brief:
    *   get the token of self, the token is self's pagetable frame's starting physical address
    * @retval:
    *   self token
    */
    #[cfg(target_arch = "riscv32")]
    fn token(&self) -> usize {
        self.root_frame.number() | (1 << 31) // as satp
    }
    #[cfg(target_arch = "riscv64")]
    fn token(&self) -> usize {
        0 // TODO
    }

    fn alloc_frame() -> Option<usize> {
        alloc_frame()
    }

    fn dealloc_frame(target: usize) {
        dealloc_frame(target)
    }
}

#[cfg(target_arch = "riscv32")]
const SATP_MODE: satp::Mode = satp::Mode::Sv32;
#[cfg(target_arch = "riscv64")]
const SATP_MODE: satp::Mode = satp::Mode::Sv48;

impl InactivePageTable0 {
    /*
    * @brief:
    *   map the kernel code memory address (p2 page table) in the new inactive page table according the current active page table
    */
    #[cfg(target_arch = "riscv32")]
    fn map_kernel(&mut self) {
        let table = unsafe { &mut *ROOT_PAGE_TABLE };
        let e0 = table[0x40];
        let e1 = table[KERNEL_P2_INDEX];
        assert!(!e1.is_unused());
        // for larger heap memroy
        let e2 = table[KERNEL_P2_INDEX + 1];
        assert!(!e2.is_unused());
        let e3 = table[KERNEL_P2_INDEX + 2];
        assert!(!e2.is_unused());

        self.edit(|_| {
            table[0x40] = e0;
            table[KERNEL_P2_INDEX].set(e1.frame(), EF::VALID | EF::GLOBAL);
            // for larger heap memroy
            table[KERNEL_P2_INDEX + 1].set(e2.frame(), EF::VALID | EF::GLOBAL);
            table[KERNEL_P2_INDEX + 2].set(e3.frame(), EF::VALID | EF::GLOBAL);
        });
    }
    #[cfg(target_arch = "riscv64")]
    fn map_kernel(&mut self) {
        // TODO
    }
}

impl Drop for InactivePageTable0 {
    fn drop(&mut self) {
        info!("PageTable dropping: {:?}", self);
        Self::dealloc_frame(self.root_frame.start_address().as_usize());
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
