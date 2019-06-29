//! Shared memory & Copy-on-write extension for page table
//!
//! To use the CowExt, make a wrapper over the original apge table
//! Like: CowExt::new(origin_page_table)
//! Invoke page_fault_handler() on the CowExt to run the COW process
//! If the method above returns true, the COW process is executed, else do your own things.
//!
//! To implement the CowExt, we added a "shared state" to the page table entry
//! We use 2bits in the entry for "readonly and shared" and "writable and shared"
//! For CPU, the page of the entry is present and readonly,
//! and it's possible to read the page through different page tables
//! but when the page is writen, the page fault will be triggered.
//! When page fault is triggered, the page_fault_handler() on the CowExt should be invoked.
//! In the page_fault_handler() method, we return false if the page is accurately readonly.
//! Elsewise we copy the data in the page into a newly allocated frame,
//! and modify the page table entry to map the page to the frame, and set the present and writable bit.
//!
//! A frame can have write and read reference at the same time,
//! so we need to maintain the count of write and read reference.
//! When page fault occurs, if the read reference count is 0 and the write reference count is 1，
//! The copy process should be skipped and the entry is mark as writable directly.

use super::paging::*;
use super::*;
use alloc::collections::BTreeMap;
use core::ops::{Deref, DerefMut};

/// Wrapper for page table, supporting shared map & copy-on-write
pub struct CowExt<T: PageTable> {
    page_table: T,
    rc_map: FrameRcMap,
}

impl<T: PageTable> CowExt<T> {
    /*
     **  @brief  create a COW extension
     **  @param  page_table: T        the inner page table
     **  @retval CowExt               the COW extension created
     */
    pub fn new(page_table: T) -> Self {
        CowExt {
            page_table,
            rc_map: FrameRcMap::default(),
        }
    }
    /*
     **  @brief  map the virtual address to a target physics address as shared
     **  @param  addr: VirtAddr       the virual address to map
     **  @param  target: VirtAddr     the target physics address
     **  @param  writable: bool       if it is true, set the page as writable and shared
     **                               else set the page as readonly and shared
     **  @retval none
     */
    pub fn map_to_shared(&mut self, addr: VirtAddr, target: PhysAddr, writable: bool) {
        let entry = self.page_table.map(addr, target);
        entry.set_writable(false);
        entry.set_shared(writable);
        entry.update();
        let frame = target / PAGE_SIZE;
        match writable {
            true => self.rc_map.write_increase(&frame),
            false => self.rc_map.read_increase(&frame),
        }
    }
    /*
     **  @brief  unmap a virual address from physics address
     **          with apecial additional process for shared page
     **  @param  addr: VirtAddr       the virual address to unmap
     **  @retval none
     */
    pub fn unmap_shared(&mut self, addr: VirtAddr) {
        let entry = self.page_table.get_entry(addr).expect("entry not exist");
        let frame = entry.target() / PAGE_SIZE;
        if entry.readonly_shared() {
            self.rc_map.read_decrease(&frame);
        } else if entry.writable_shared() {
            self.rc_map.write_decrease(&frame);
        }
        self.page_table.unmap(addr);
    }
    /*
     **  @brief  execute the COW process for page fault
     **          This function must be called whenever PageFault happens.
     **  @param  addr: VirtAddr       the virual address of the page fault
     **  @param  alloc_frame: impl FnOnce() -> PhysAddr
     **                               the page allocation function
     **                               that allocate a page and returns physics address
     **                               of beginning of the page
     **  @retval bool                 whether copy-on-write happens.
     */
    pub fn page_fault_handler(
        &mut self,
        addr: VirtAddr,
        alloc_frame: impl FnOnce() -> PhysAddr,
    ) -> bool {
        let entry = self.page_table.get_entry(addr);
        if entry.is_none() {
            return false;
        }
        let entry = entry.unwrap();
        if !entry.readonly_shared() && !entry.writable_shared() {
            return false;
        }
        let frame = entry.target() / PAGE_SIZE;
        if self.rc_map.read_count(&frame) == 0 && self.rc_map.write_count(&frame) == 1 {
            entry.clear_shared();
            entry.set_writable(true);
            entry.update();
            self.rc_map.write_decrease(&frame);
            return true;
        }
        use core::mem::MaybeUninit;
        let mut temp_data: [u8; PAGE_SIZE] = unsafe { MaybeUninit::zeroed().assume_init() };
        temp_data[..].copy_from_slice(self.get_page_slice_mut(addr));

        self.unmap_shared(addr);
        self.map(addr, alloc_frame());

        self.get_page_slice_mut(addr)
            .copy_from_slice(&temp_data[..]);
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
///
/// It will lazily construct the `BTreeMap`, to avoid heap alloc when heap is unavailable.
#[derive(Default)]
struct FrameRcMap(Option<BTreeMap<Frame, (u16, u16)>>);

type Frame = usize;

impl FrameRcMap {
    /*
     **  @brief  get the read reference count of the frame
     **  @param  frame: &Frame        the frame to get the read reference count
     **  @retval u16                  the read reference count
     */
    fn read_count(&mut self, frame: &Frame) -> u16 {
        self.map().get(frame).unwrap_or(&(0, 0)).0
    }
    /*
     **  @brief  get the write reference count of the frame
     **  @param  frame: &Frame        the frame to get the write reference count
     **  @retval u16                  the write reference count
     */
    fn write_count(&mut self, frame: &Frame) -> u16 {
        self.map().get(frame).unwrap_or(&(0, 0)).1
    }
    /*
     **  @brief  increase the read reference count of the frame
     **  @param  frame: &Frame        the frame to increase the read reference count
     **  @retval none
     */
    fn read_increase(&mut self, frame: &Frame) {
        let (r, w) = self.map().get(&frame).unwrap_or(&(0, 0)).clone();
        self.map().insert(frame.clone(), (r + 1, w));
    }
    /*
     **  @brief  decrease the read reference count of the frame
     **  @param  frame: &Frame        the frame to decrease the read reference count
     **  @retval none
     */
    fn read_decrease(&mut self, frame: &Frame) {
        self.map().get_mut(frame).unwrap().0 -= 1;
    }
    /*
     **  @brief  increase the write reference count of the frame
     **  @param  frame: &Frame        the frame to increase the write reference count
     **  @retval none
     */
    fn write_increase(&mut self, frame: &Frame) {
        let (r, w) = self.map().get(&frame).unwrap_or(&(0, 0)).clone();
        self.map().insert(frame.clone(), (r, w + 1));
    }
    /*
     **  @brief  decrease the write reference count of the frame
     **  @param  frame: &Frame        the frame to decrease the write reference count
     **  @retval none
     */
    fn write_decrease(&mut self, frame: &Frame) {
        self.map().get_mut(frame).unwrap().1 -= 1;
    }
    /*
     **  @brief  get the internal btree map, lazily initialize the btree map if it is not present
     **  @retval &mut BTreeMap<Frame, (u16, u16)>
     **                               the internal btree map
     */
    fn map(&mut self) -> &mut BTreeMap<Frame, (u16, u16)> {
        if self.0.is_none() {
            self.0 = Some(BTreeMap::new());
        }
        self.0.as_mut().unwrap()
    }
}

pub mod test {
    use super::*;

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

        pt.page_table
            .set_handler(Box::new(move |_, addr: VirtAddr| {
                pt0.page_fault_handler(addr, || alloc.alloc());
            }));

        test_with(&mut pt);
    }

    pub fn test_with(pt: &mut CowExt<impl PageTable>) {
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
        assert_ne!(pt.get_entry(0x1000).unwrap().target(), target);
        assert_eq!(pt.read(0x1000), 2);
        assert_eq!(pt.read(0x2000), 1);
        assert_eq!(pt.read(0x3000), 1);

        pt.unmap_shared(0x3000);
        assert_eq!(pt.rc_map.read_count(&frame), 0);
        assert_eq!(pt.rc_map.write_count(&frame), 1);
        // assert!(!pt.get_entry(0x3000).present());

        pt.write(0x2000, 3);
        assert_eq!(pt.rc_map.read_count(&frame), 0);
        assert_eq!(pt.rc_map.write_count(&frame), 0);
        assert_eq!(
            pt.get_entry(0x2000).unwrap().target(),
            target,
            "The last write reference should not allocate new frame."
        );
        assert_eq!(pt.read(0x1000), 2);
        assert_eq!(pt.read(0x2000), 3);
    }
}
