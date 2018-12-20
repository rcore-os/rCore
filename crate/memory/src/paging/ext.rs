//! Helper functions

use super::*;

pub trait PageTableExt: PageTable {
    const TEMP_PAGE_ADDR: VirtAddr = 0xcafeb000;

    fn with_temporary_map<T, D>(&mut self, target: PhysAddr, f: impl FnOnce(&mut Self, &mut D) -> T) -> T {
        self.map(Self::TEMP_PAGE_ADDR, target);
        let data = unsafe { &mut *(self.get_page_slice_mut(Self::TEMP_PAGE_ADDR).as_ptr() as *mut D) };
        let ret = f(self, data);
        self.unmap(Self::TEMP_PAGE_ADDR);
        ret
    }
}