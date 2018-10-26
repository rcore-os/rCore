//! Page table implementations for aarch64.

use ucore_memory::memory_set::*;
use ucore_memory::paging::*;

type VirtAddr = usize;
type PhysAddr = usize;

/// TODO
pub struct ActivePageTable {
    // TODO
}

impl ActivePageTable {
    /// TODO
    pub unsafe fn new() -> Self {
        unimplemented!()
    }
}

impl PageTable for ActivePageTable {
    type Entry = PageEntry;

    fn map(&mut self, addr: VirtAddr, target: PhysAddr) -> &mut Self::Entry {
        unimplemented!()
    }
    fn unmap(&mut self, addr: VirtAddr) {
        unimplemented!()
    }

    fn get_entry(&mut self, addr: VirtAddr) -> &mut Self::Entry {
        unimplemented!()
    }

    // For testing with mock
    fn get_page_slice_mut<'a,'b>(&'a mut self, addr: VirtAddr) -> &'b mut [u8] {
        unimplemented!()
    }

    fn read(&mut self, addr: VirtAddr) -> u8 {
        unimplemented!()
    }

    fn write(&mut self, addr: VirtAddr, data: u8) {
        unimplemented!()
    }
}

/// TODO
pub struct PageEntry {
    // TODO
}

impl Entry for PageEntry {
    /// IMPORTANT!
    /// This must be called after any change to ensure it become effective.
    /// Usually this will make a flush to TLB/MMU.
    fn update(&mut self) {
        unimplemented!()
    }

    /// Will be set when accessed
    fn accessed(&self) -> bool {
        unimplemented!()
    }

    /// Will be set when written
    fn dirty(&self) -> bool {
        unimplemented!()
    }

    /// Will PageFault when try to write page where writable=0
    fn writable(&self) -> bool {
        unimplemented!()
    }

    /// Will PageFault when try to access page where present=0
    fn present(&self) -> bool {
        unimplemented!()
    }


    fn clear_accessed(&mut self) {
        unimplemented!()
    }

    fn clear_dirty(&mut self) {
        unimplemented!()
    }

    fn set_writable(&mut self, value: bool) {
        unimplemented!()
    }

    fn set_present(&mut self, value: bool) {
        unimplemented!()
    }


    fn target(&self) -> PhysAddr {
        unimplemented!()
    }

    fn set_target(&mut self, target: PhysAddr) {
        unimplemented!()
    }


    // For Copy-on-write extension
    fn writable_shared(&self) -> bool {
        unimplemented!()
    }

    fn readonly_shared(&self) -> bool {
        unimplemented!()
    }

    fn set_shared(&mut self, writable: bool) {
        unimplemented!()
    }

    fn clear_shared(&mut self) {
        unimplemented!()
    }


    // For Swap extension
    fn swapped(&self) -> bool {
        unimplemented!()
    }

    fn set_swapped(&mut self, value: bool) {
        unimplemented!()
    }


    fn user(&self) -> bool {
        unimplemented!()
    }

    fn set_user(&mut self, value: bool) {
        unimplemented!()
    }

    fn execute(&self) -> bool {
        unimplemented!()
    }

    fn set_execute(&mut self, value: bool) {
        unimplemented!()
    }

}

/// TODO
pub struct InactivePageTable0 {
    // TODO
}

/// TODO
impl InactivePageTable for InactivePageTable0 {
    type Active = ActivePageTable;

        fn new() -> Self {
        unimplemented!()
    }

    fn new_bare() -> Self {
        unimplemented!()
    }

    fn edit(&mut self, f: impl FnOnce(&mut Self::Active)) {
        unimplemented!()
    }

    unsafe fn activate(&self) {
        unimplemented!()
    }

    unsafe fn with(&self, f: impl FnOnce()) {
        unimplemented!()
    }

    fn token(&self) -> usize {
        unimplemented!()
    }

    fn alloc_frame() -> Option<PhysAddr> {
        unimplemented!()
    }

    fn dealloc_frame(target: PhysAddr) {
        unimplemented!()
    }

    fn alloc_stack() -> Stack {
        unimplemented!()
    }
}
