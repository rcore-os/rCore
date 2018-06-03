//! Shared memory & Copy-on-write extension for page table
//!
//! 利用x86页表项的特性，实现共享内存和写时复制机制。
//!
//! ## 使用说明
//!
//! 实现目标机制的全部代码都在此文件中，只对原始代码进行了极小的微调和补充。
//! 使用时直接 use Trait ，调用相应函数即可。
//! 此外需要在PageFault时，调用`try_copy_on_write()`，如返回true，说明发生了COW，否则再进行其他处理。
//!
//! ## 实现概述
//!
//! 我们为页表项定义一个新的状态：共享态。
//! 在这一状态下，对于CPU而言它是存在+只读的，可以通过不同的页表对该页进行读操作。
//! 当进行写操作时，会触发PageFault。如果此页实际是只读的，则正常抛出异常。
//! 否则如果实际是可写的，此时再新分配一个物理页，复制数据，将页表项指向该页，并置为存在+可写。
//!
//! 对于同一个物理页，允许同时存在读引用和写引用，为此我们需要维护二者的引用计数。
//! 当PageFault时，如果读引用为0，写引用为1，则直接标记可写。
//!
//! ## 各标记位状态
//!
//! * bit 9-11: 用来识别当前状态，值为001表示只读共享，010表示可写共享
//! * bit 0: 存在位，值为1
//! * bit 1: 可写位，值为0
//!
//! ## 实现细节
//!
//! * Trait `EntryCowExt` 为页表项定义了辅助函数
//!
//! * Trait `PageTableCowExt` 为活跃页表定义了操作共享映射的接口函数
//!   其中 `cow_to_owned()` 是发生PageFault时的处理入口
//!   注意此处的实现对象是 `ActivePageTable`，因为当写入时需要读取目标页的数据
//!
//! * 为了维护引用计数，开一个全局映射 `RC_MAP`: Frame -> (read_count, write_count)

use alloc::BTreeMap;
pub use self::test::test_cow;
use spin::Mutex;
use super::*;
use x86_64::instructions::tlb;
use x86_64::VirtualAddress;

trait EntryCowExt {
    fn is_shared(&self) -> bool;
    fn is_cow(&self) -> bool;
    fn set_shared(&mut self, frame: Frame, flags: EntryFlags);
    fn copy_on_write(&mut self, new_frame: Option<Frame>);
    fn reset(&mut self);
}

pub trait PageTableCowExt {
    fn map_to_shared(&mut self, page: Page, frame: Frame, flags: EntryFlags);
    fn unmap_shared(&mut self, page: Page);
    fn try_copy_on_write(&mut self, addr: VirtAddr) -> bool;
}

impl EntryCowExt for Entry {
    fn is_shared(&self) -> bool {
        self.flags().contains(EntryFlags::SHARED)
    }
    fn is_cow(&self) -> bool {
        self.flags().contains(EntryFlags::COW)
    }
    fn set_shared(&mut self, frame: Frame, mut flags: EntryFlags) {
        flags |= EntryFlags::PRESENT;
        if flags.contains(EntryFlags::WRITABLE) {
            flags.remove(EntryFlags::WRITABLE);
            flags.insert(EntryFlags::COW);
            RC_MAP.write_increase(&frame);
        } else {
            flags.insert(EntryFlags::SHARED);
            RC_MAP.read_increase(&frame);
        }
        self.set(frame, flags);
    }
    fn copy_on_write(&mut self, new_frame: Option<Frame>) {
        //  assert!(self.is_cow());
        let frame = self.pointed_frame().unwrap();
        RC_MAP.write_decrease(&frame);
        let mut flags = self.flags() | EntryFlags::WRITABLE;
        flags.remove(EntryFlags::COW);
        self.set(new_frame.unwrap_or(frame), flags);
    }
    fn reset(&mut self) {
        let frame = self.pointed_frame().unwrap();
        if self.is_shared() {
            RC_MAP.read_decrease(&frame);
        } else if self.is_cow() {
            RC_MAP.write_decrease(&frame);
        }
        self.set_unused();
    }
}

impl PageTableCowExt for ActivePageTable {
    fn map_to_shared(&mut self, page: Page, frame: Frame, flags: EntryFlags) {
        let entry = self.entry_mut(page);
        assert!(entry.is_unused());
        entry.set_shared(frame, flags);
    }
    fn unmap_shared(&mut self, page: Page) {
        self.entry_mut(page).reset();
        tlb::flush(VirtualAddress(page.start_address()));
    }
    fn try_copy_on_write(&mut self, addr: VirtAddr) -> bool {
        let page = Page::of_addr(addr);
        let entry = self.entry_mut(page);
        if !entry.is_cow() {
            return false;
        }
        let frame = entry.pointed_frame().unwrap();
        if RC_MAP.read_count(&frame) == 0 && RC_MAP.write_count(&frame) == 1 {
            entry.copy_on_write(None);
        } else {
            use core::{slice, mem::uninitialized};
            let mut temp_data: [u8; PAGE_SIZE] = unsafe { uninitialized() };
            let page_data = unsafe { slice::from_raw_parts_mut(page.start_address() as *mut u8, PAGE_SIZE) };
            temp_data.copy_from_slice(page_data);

            entry.copy_on_write(Some(alloc_frame()));
            tlb::flush(VirtualAddress(page.start_address()));

            page_data.copy_from_slice(&temp_data);
        }
        true
    }
}

/// A global map contains reference count for shared frame
lazy_static! {
    static ref RC_MAP: FrameRcMap = FrameRcMap::new();
}
struct FrameRcMap(Mutex<BTreeMap<Frame, (u8, u8)>>);

impl FrameRcMap {
    fn new() -> Self {
        FrameRcMap(Mutex::new(BTreeMap::new()))
    }
    fn read_count(&self, frame: &Frame) -> u8 {
        self.0.lock().get(frame).unwrap_or(&(0, 0)).0
    }
    fn write_count(&self, frame: &Frame) -> u8 {
        self.0.lock().get(frame).unwrap_or(&(0, 0)).1
    }
    fn read_increase(&self, frame: &Frame) {
        let mut map = self.0.lock();
        let (r, w) = map.get(&frame).unwrap_or(&(0, 0)).clone();
        map.insert(frame.clone(), (r + 1, w));
    }
    fn read_decrease(&self, frame: &Frame) {
        let mut map = self.0.lock();
        map.get_mut(frame).unwrap().0 -= 1;
    }
    fn write_increase(&self, frame: &Frame) {
        let mut map = self.0.lock();
        let (r, w) = map.get(&frame).unwrap_or(&(0, 0)).clone();
        map.insert(frame.clone(), (r, w + 1));
    }
    fn write_decrease(&self, frame: &Frame) {
        let mut map = self.0.lock();
        map.get_mut(frame).unwrap().1 -= 1;
    }
}

mod test {
    use super::*;

    pub fn test_cow() {
        let mut page_table = unsafe { ActivePageTable::new() };
        let frame = alloc_frame();

        page_table.map_to(Page::of_addr(0x1000), frame.clone(), EntryFlags::WRITABLE);
        unsafe { *(0x1000 as *mut u8) = 1; }
        assert_eq!(unsafe { *(0x1000 as *const u8) }, 1);
        page_table.unmap(Page::of_addr(0x1000));

        page_table.map_to_shared(Page::of_addr(0x1000), frame.clone(), EntryFlags::WRITABLE);
        page_table.map_to_shared(Page::of_addr(0x2000), frame.clone(), EntryFlags::WRITABLE);
        page_table.map_to_shared(Page::of_addr(0x3000), frame.clone(), EntryFlags::PRESENT);
        assert_eq!(RC_MAP.read_count(&frame), 1);
        assert_eq!(RC_MAP.write_count(&frame), 2);
        assert_eq!(unsafe { *(0x1000 as *const u8) }, 1);
        assert_eq!(unsafe { *(0x2000 as *const u8) }, 1);
        assert_eq!(unsafe { *(0x3000 as *const u8) }, 1);

        unsafe { *(0x1000 as *mut u8) = 2; }
        assert_eq!(RC_MAP.read_count(&frame), 1);
        assert_eq!(RC_MAP.write_count(&frame), 1);
        assert_ne!(page_table.translate_page(Page::of_addr(0x1000)).unwrap(), frame);
        assert_eq!(unsafe { *(0x1000 as *const u8) }, 2);
        assert_eq!(unsafe { *(0x2000 as *const u8) }, 1);
        assert_eq!(unsafe { *(0x3000 as *const u8) }, 1);

        page_table.unmap_shared(Page::of_addr(0x3000));
        assert_eq!(RC_MAP.read_count(&frame), 0);
        assert_eq!(RC_MAP.write_count(&frame), 1);
        assert_eq!(page_table.translate_page(Page::of_addr(0x3000)), None);

        unsafe { *(0x2000 as *mut u8) = 3; }
        assert_eq!(RC_MAP.read_count(&frame), 0);
        assert_eq!(RC_MAP.write_count(&frame), 0);
        assert_eq!(page_table.translate_page(Page::of_addr(0x2000)).unwrap(), frame,
                   "The last write reference should not allocate new frame.");
        assert_eq!(unsafe { *(0x1000 as *const u8) }, 2);
        assert_eq!(unsafe { *(0x2000 as *const u8) }, 3);
    }
}