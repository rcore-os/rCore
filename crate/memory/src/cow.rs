//! Shared memory & Copy-on-write extension for page table

use super::paging::*;
use super::*;
use alloc::BTreeMap;
use core::ops::{Deref, DerefMut};

/// Wrapper for page table, supporting shared map & copy-on-write
struct CowExt<T: PageTable> {
    page_table: T,
    rc_map: FrameRcMap,
}

impl<T: PageTable> CowExt<T> {
    pub fn new(page_table: T) -> Self {
        CowExt {
            page_table,
            rc_map: FrameRcMap::default(),
        }
    }
    pub fn map_to_shared(&mut self, addr: VirtAddr, target: PhysAddr, writable: bool) {
        let entry = self.page_table.map(addr, target);
        entry.set_writable(false);
        entry.set_shared(writable);
        let frame = target / PAGE_SIZE;
        match writable {
            true => self.rc_map.write_increase(&frame),
            false => self.rc_map.read_increase(&frame),
        }
    }
    pub fn unmap_shared(&mut self, addr: VirtAddr) {
        {
            let entry = self.page_table.get_entry(addr);
            let frame = entry.target() / PAGE_SIZE;
            if entry.readonly_shared() {
                self.rc_map.read_decrease(&frame);
            } else if entry.writable_shared() {
                self.rc_map.write_decrease(&frame);
            }
        }
        self.page_table.unmap(addr);
    }
    /// This function must be called whenever PageFault happens.
    /// Return whether copy-on-write happens.
    pub fn page_fault_handler(&mut self, addr: VirtAddr, alloc_frame: impl FnOnce() -> PhysAddr) -> bool {
        {
            let entry = self.page_table.get_entry(addr);
            if !entry.readonly_shared() && !entry.writable_shared() {
                return false;
            }
            let frame = entry.target() / PAGE_SIZE;
            if self.rc_map.read_count(&frame) == 0 && self.rc_map.write_count(&frame) == 1 {
                entry.clear_shared();
                entry.set_writable(true);
                self.rc_map.write_decrease(&frame);
                return true;
            }
        }
        use core::mem::uninitialized;
        let mut temp_data: [u8; PAGE_SIZE] = unsafe { uninitialized() };
        self.read_page(addr, &mut temp_data[..]);

        self.unmap_shared(addr);
        self.map(addr, alloc_frame());

        self.write_page(addr, &temp_data[..]);
        true
    }
}

impl<T: PageTable> Deref for CowExt<T> {
    type Target = T;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.page_table
    }
}

impl<T: PageTable> DerefMut for CowExt<T> {
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.page_table
    }
}

/// A map contains reference count for shared frame
#[derive(Default)]
struct FrameRcMap(BTreeMap<Frame, (u8, u8)>);

type Frame = usize;

impl FrameRcMap {
    fn read_count(&mut self, frame: &Frame) -> u8 {
        self.0.get(frame).unwrap_or(&(0, 0)).0
    }
    fn write_count(&mut self, frame: &Frame) -> u8 {
        self.0.get(frame).unwrap_or(&(0, 0)).1
    }
    fn read_increase(&mut self, frame: &Frame) {
        let (r, w) = self.0.get(&frame).unwrap_or(&(0, 0)).clone();
        self.0.insert(frame.clone(), (r + 1, w));
    }
    fn read_decrease(&mut self, frame: &Frame) {
        self.0.get_mut(frame).unwrap().0 -= 1;
    }
    fn write_increase(&mut self, frame: &Frame) {
        let (r, w) = self.0.get(&frame).unwrap_or(&(0, 0)).clone();
        self.0.insert(frame.clone(), (r, w + 1));
    }
    fn write_decrease(&mut self, frame: &Frame) {
        self.0.get_mut(frame).unwrap().1 -= 1;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::boxed::Box;

    #[test]
    fn test() {
        let mut pt = CowExt::new(MockPageTable::new());
        let pt0 = unsafe { &mut *(&mut pt as *mut CowExt<MockPageTable>) };

        struct FrameAlloc(usize);
        impl FrameAlloc {
            fn alloc(&mut self) -> PhysAddr {
                let pa = self.0 * PAGE_SIZE;
                self.0 += 1;
                pa
            }
        }
        let mut alloc = FrameAlloc(4);

        pt.page_table.set_handler(Box::new(move |_, addr: VirtAddr| {
            pt0.page_fault_handler(addr, || alloc.alloc());
        }));
        let target = 0x0;
        let frame = 0x0;

        pt.map(0x1000, target);
        pt.write(0x1000, 1);
        assert_eq!(pt.read(0x1000), 1);
        pt.unmap(0x1000);

        pt.map_to_shared(0x1000, target, true);
        pt.map_to_shared(0x2000, target, true);
        pt.map_to_shared(0x3000, target, false);
        assert_eq!(pt.rc_map.read_count(&frame), 1);
        assert_eq!(pt.rc_map.write_count(&frame), 2);
        assert_eq!(pt.read(0x1000), 1);
        assert_eq!(pt.read(0x2000), 1);
        assert_eq!(pt.read(0x3000), 1);

        pt.write(0x1000, 2);
        assert_eq!(pt.rc_map.read_count(&frame), 1);
        assert_eq!(pt.rc_map.write_count(&frame), 1);
        assert_ne!(pt.get_entry(0x1000).target(), target);
        assert_eq!(pt.read(0x1000), 2);
        assert_eq!(pt.read(0x2000), 1);
        assert_eq!(pt.read(0x3000), 1);

        pt.unmap_shared(0x3000);
        assert_eq!(pt.rc_map.read_count(&frame), 0);
        assert_eq!(pt.rc_map.write_count(&frame), 1);
        assert!(!pt.get_entry(0x3000).present());

        pt.write(0x2000, 3);
        assert_eq!(pt.rc_map.read_count(&frame), 0);
        assert_eq!(pt.rc_map.write_count(&frame), 0);
        assert_eq!(pt.get_entry(0x2000).target(), target,
                   "The last write reference should not allocate new frame.");
        assert_eq!(pt.read(0x1000), 2);
        assert_eq!(pt.read(0x2000), 3);
    }
}