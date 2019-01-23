//! Generic page table interface
//!
//! Implemented for every architecture, used by OS.

use super::*;
#[cfg(test)]
pub use self::mock_page_table::MockPageTable;
pub use self::ext::*;

#[cfg(test)]
mod mock_page_table;
mod ext;


pub trait PageTable {
//    type Entry: Entry;

    /// Map a page of virual address `addr` to the frame of physics address `target`
    /// Return the page table entry of the mapped virual address
    fn map(&mut self, addr: VirtAddr, target: PhysAddr) -> &mut Entry;

    /// Unmap a page of virual address `addr`
    fn unmap(&mut self, addr: VirtAddr);

    /// Get the page table entry of a page of virual address `addr`
    /// If its page do not exist, return `None`
    fn get_entry(&mut self, addr: VirtAddr) -> Option<&mut Entry>;

    /// Get a mutable reference of the content of a page of virtual address `addr`
    /// Used for testing with mock
    fn get_page_slice_mut<'a>(&mut self, addr: VirtAddr) -> &'a mut [u8] {
        unsafe { core::slice::from_raw_parts_mut((addr & !(PAGE_SIZE - 1)) as *mut u8, PAGE_SIZE) }
    }
    /// Read data from virtual address `addr`
    /// Used for testing with mock
    fn read(&mut self, addr: VirtAddr) -> u8 {
        unsafe { (addr as *const u8).read() }
    }

    /// Write data to virtual address `addr`
    /// Used for testing with mock
    fn write(&mut self, addr: VirtAddr, data: u8) {
        unsafe { (addr as *mut u8).write(data) }
    }

    /// When 'vaddr' is not mapped, map it to 'paddr'.
    fn map_if_not_exists(&mut self, vaddr: VirtAddr, paddr: usize) -> bool {
        if let None = self.get_entry(vaddr) {
            self.map(vaddr, paddr);
            true
        } else {
            false
        }
    }
}

/// Page Table Entry
pub trait Entry {
    /// Make all changes take effect.
    ///
    /// IMPORTANT!
    /// This must be called after any change to ensure it become effective.
    /// Usually it will cause a TLB/MMU flush.
    fn update(&mut self);
    /// A bit set by hardware when the page is accessed
    fn accessed(&self) -> bool;
    /// A bit set by hardware when the page is written
    fn dirty(&self) -> bool;
    /// Will PageFault when try to write page where writable=0
    fn writable(&self) -> bool;
    /// Will PageFault when try to access page where present=0
    fn present(&self) -> bool;

    fn clear_accessed(&mut self);
    fn clear_dirty(&mut self);
    fn set_writable(&mut self, value: bool);
    fn set_present(&mut self, value: bool);

    /// The target physics address in the entry
    /// Can be used for other purpose if present=0
    fn target(&self) -> PhysAddr;
    fn set_target(&mut self, target: PhysAddr);

    // For Copy-on-write
    fn writable_shared(&self) -> bool;
    fn readonly_shared(&self) -> bool;
    fn set_shared(&mut self, writable: bool);
    fn clear_shared(&mut self);

    // For Swap
    fn swapped(&self) -> bool;
    fn set_swapped(&mut self, value: bool);

    fn user(&self) -> bool;
    fn set_user(&mut self, value: bool);
    fn execute(&self) -> bool;
    fn set_execute(&mut self, value: bool);
    fn mmio(&self) -> u8;
    fn set_mmio(&mut self, value: u8);
}

/// An inactive page table
/// Note: InactivePageTable is not a PageTable
///       but it can be activated and "become" a PageTable
pub trait InactivePageTable: Sized {
    /// the active version of page table
    type Active: PageTable;

    /// Create a new page table with kernel memory mapped
    fn new() -> Self {
        let mut pt = Self::new_bare();
        pt.map_kernel();
        pt
    }

    /// Create a new page table without kernel memory mapped
    fn new_bare() -> Self;

    /// Map kernel segments
    fn map_kernel(&mut self);

    /// CR3 on x86, SATP on RISCV, TTBR on AArch64
    fn token(&self) -> usize;
    unsafe fn set_token(token: usize);
    fn active_token() -> usize;
    fn flush_tlb();

    /// Make this page table editable
    /// Set the recursive entry of current active page table to this
    fn edit<T>(&mut self, f: impl FnOnce(&mut Self::Active) -> T) -> T;

    /// Activate this page table
    unsafe fn activate(&self) {
        let old_token = Self::active_token();
        let new_token = self.token();
        debug!("switch table {:x?} -> {:x?}", old_token, new_token);
        if old_token != new_token {
            Self::set_token(new_token);
            Self::flush_tlb();
        }
    }

    /// Execute function `f` with this page table activated
    unsafe fn with<T>(&self, f: impl FnOnce() -> T) -> T {
        let old_token = Self::active_token();
        let new_token = self.token();
        debug!("switch table {:x?} -> {:x?}", old_token, new_token);
        if old_token != new_token {
            Self::set_token(new_token);
            Self::flush_tlb();
        }
        let ret = f();
        debug!("switch table {:x?} -> {:x?}", new_token, old_token);
        if old_token != new_token {
            Self::set_token(old_token);
            Self::flush_tlb();
        }
        ret
    }
}
