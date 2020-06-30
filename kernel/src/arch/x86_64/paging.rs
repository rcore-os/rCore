use super::consts::*;
use crate::memory::{alloc_frame, dealloc_frame, phys_to_virt};
use core::mem::ManuallyDrop;
use log::*;
use rcore_memory::paging::*;
use x86_64::instructions::tlb;
use x86_64::registers::control::{Cr3, Cr3Flags};
use x86_64::structures::paging::{
    frame::PhysFrame as Frame,
    mapper::{MappedPageTable, Mapper},
    page::{Page, PageRange, Size4KiB},
    page_table::{PageTable as x86PageTable, PageTableEntry, PageTableFlags as EF},
    FrameAllocator, FrameDeallocator,
};
use x86_64::{PhysAddr, VirtAddr};

pub trait PageExt {
    fn of_addr(address: usize) -> Self;
    fn range_of(begin: usize, end: usize) -> PageRange;
}

impl PageExt for Page {
    fn of_addr(address: usize) -> Self {
        Page::containing_address(VirtAddr::new(address as u64))
    }
    fn range_of(begin: usize, end: usize) -> PageRange {
        Page::range(Page::of_addr(begin), Page::of_addr(end - 1) + 1)
    }
}

pub trait FrameExt {
    fn of_addr(address: usize) -> Self;
}

impl FrameExt for Frame {
    fn of_addr(address: usize) -> Self {
        Frame::containing_address(PhysAddr::new(address as u64))
    }
}

pub struct PageTableImpl(
    MappedPageTable<'static, fn(Frame) -> *mut x86PageTable>,
    Option<PageEntry>,
    Frame,
);

pub struct PageEntry(&'static mut PageTableEntry, Page, Frame);

impl PageTable for PageTableImpl {
    fn map(&mut self, addr: usize, target: usize) -> &mut dyn Entry {
        let flags = EF::PRESENT | EF::WRITABLE | EF::NO_EXECUTE;
        unsafe {
            self.0
                .map_to(
                    Page::of_addr(addr),
                    Frame::of_addr(target),
                    flags,
                    &mut FrameAllocatorForX86,
                )
                .unwrap()
                .flush();
        }
        flush_tlb_all(addr);
        self.get_entry(addr).unwrap()
    }

    fn unmap(&mut self, addr: usize) {
        self.0.unmap(Page::of_addr(addr)).unwrap().1.flush();
        flush_tlb_all(addr);
    }

    fn get_entry(&mut self, addr: usize) -> Option<&mut dyn Entry> {
        let mut page_table = frame_to_page_table(self.2);
        for level in 0..4 {
            let index = (addr >> (12 + (3 - level) * 9)) & 0o777;
            let entry = unsafe { &mut (&mut *page_table)[index] };
            if level == 3 {
                let page = Page::of_addr(addr);
                self.1 = Some(PageEntry(entry, page, self.2));
                return Some(self.1.as_mut().unwrap());
            }
            if !entry.flags().contains(EF::PRESENT) {
                return None;
            }
            page_table = frame_to_page_table(entry.frame().unwrap());
        }
        unreachable!();
    }

    fn get_page_slice_mut<'a>(&mut self, addr: usize) -> &'a mut [u8] {
        let frame = self.0.translate_page(Page::of_addr(addr)).unwrap();
        let vaddr = phys_to_virt(frame.start_address().as_u64() as usize);
        unsafe { core::slice::from_raw_parts_mut(vaddr as *mut u8, 0x1000) }
    }

    fn flush_cache_copy_user(&mut self, _start: usize, _end: usize, _execute: bool) {}
}

fn frame_to_page_table(frame: Frame) -> *mut x86PageTable {
    let vaddr = phys_to_virt(frame.start_address().as_u64() as usize);
    vaddr as *mut x86PageTable
}

impl Entry for PageEntry {
    fn update(&mut self) {
        use x86_64::instructions::tlb::flush;
        let addr = self.1.start_address();
        flush(addr);
        flush_tlb_all(addr.as_u64() as usize);
    }
    fn accessed(&self) -> bool {
        self.0.flags().contains(EF::ACCESSED)
    }
    fn dirty(&self) -> bool {
        self.0.flags().contains(EF::DIRTY)
    }
    fn writable(&self) -> bool {
        self.0.flags().contains(EF::WRITABLE)
    }
    fn present(&self) -> bool {
        self.0.flags().contains(EF::PRESENT)
    }
    fn clear_accessed(&mut self) {
        self.as_flags().remove(EF::ACCESSED);
    }
    fn clear_dirty(&mut self) {
        self.as_flags().remove(EF::DIRTY);
    }
    fn set_writable(&mut self, value: bool) {
        self.as_flags().set(EF::WRITABLE, value);
    }
    fn set_present(&mut self, value: bool) {
        self.as_flags().set(EF::PRESENT, value);
    }
    fn target(&self) -> usize {
        self.0.addr().as_u64() as usize
    }
    fn set_target(&mut self, target: usize) {
        let flags = self.0.flags();
        self.0.set_addr(PhysAddr::new(target as u64), flags);
    }
    fn writable_shared(&self) -> bool {
        self.0.flags().contains(EF::BIT_10)
    }
    fn readonly_shared(&self) -> bool {
        self.0.flags().contains(EF::BIT_9)
    }
    fn set_shared(&mut self, writable: bool) {
        let flags = self.as_flags();
        flags.set(EF::BIT_10, writable);
        flags.set(EF::BIT_9, !writable);
    }
    fn clear_shared(&mut self) {
        self.as_flags().remove(EF::BIT_9 | EF::BIT_10);
    }
    fn swapped(&self) -> bool {
        self.0.flags().contains(EF::BIT_11)
    }
    fn set_swapped(&mut self, value: bool) {
        self.as_flags().set(EF::BIT_11, value);
    }
    fn user(&self) -> bool {
        self.0.flags().contains(EF::USER_ACCESSIBLE)
    }
    fn set_user(&mut self, value: bool) {
        // x86_64 page table struct do not implement setting USER bit
        if value {
            let mut page_table = frame_to_page_table(self.2);
            for level in 0..4 {
                let index =
                    (self.1.start_address().as_u64() as usize >> (12 + (3 - level) * 9)) & 0o777;
                let entry = unsafe { &mut (&mut *page_table)[index] };
                entry.set_flags(entry.flags() | EF::USER_ACCESSIBLE);
                if level == 3 {
                    return;
                }
                page_table = frame_to_page_table(entry.frame().unwrap());
            }
        }
    }
    fn execute(&self) -> bool {
        !self.0.flags().contains(EF::NO_EXECUTE)
    }
    fn set_execute(&mut self, value: bool) {
        self.as_flags().set(EF::NO_EXECUTE, !value);
    }
    fn mmio(&self) -> u8 {
        0
    }
    fn set_mmio(&mut self, _value: u8) {}
}

impl PageEntry {
    fn as_flags(&mut self) -> &mut EF {
        unsafe { &mut *(self.0 as *mut _ as *mut EF) }
    }
}

impl PageTableImpl {
    /// Unsafely get the current active page table.
    /// Using ManuallyDrop to wrap the page table: this is how `core::mem::forget` is implemented now.
    pub unsafe fn active() -> ManuallyDrop<Self> {
        let frame = Cr3::read().0;
        let table = &mut *frame_to_page_table(frame);
        ManuallyDrop::new(PageTableImpl(
            MappedPageTable::new(table, frame_to_page_table),
            None,
            frame,
        ))
    }
    /// The method for getting the kernel page table.
    /// In x86_64 kernel page table and user page table are the same table. However you have to do the initialization.
    pub unsafe fn kernel_table() -> ManuallyDrop<Self> {
        Self::active()
    }
}

impl PageTableExt for PageTableImpl {
    fn new_bare() -> Self {
        let target = alloc_frame().expect("failed to allocate frame");
        let frame = Frame::of_addr(target);
        let table = unsafe { &mut *frame_to_page_table(frame) };
        table.zero();
        unsafe {
            PageTableImpl(
                MappedPageTable::new(table, frame_to_page_table),
                None,
                frame,
            )
        }
    }

    fn map_kernel(&mut self) {
        let table = unsafe { &mut *frame_to_page_table(Cr3::read().0) };
        let ekernel = table[KERNEL_PM4].clone();
        let ephysical = table[PHYSICAL_MEMORY_PM4].clone();
        let ekseg2 = table[KSEG2_PM4].clone();
        let table = unsafe { &mut *frame_to_page_table(self.2) };
        table[KERNEL_PM4].set_addr(ekernel.addr(), ekernel.flags() | EF::GLOBAL);
        table[PHYSICAL_MEMORY_PM4].set_addr(ephysical.addr(), ephysical.flags() | EF::GLOBAL);
        table[KSEG2_PM4].set_addr(ekseg2.addr(), ekseg2.flags() | EF::GLOBAL);
    }

    fn token(&self) -> usize {
        self.2.start_address().as_u64() as usize // as CR3
    }

    unsafe fn set_token(token: usize) {
        Cr3::write(
            Frame::containing_address(PhysAddr::new(token as u64)),
            Cr3Flags::empty(),
        );
    }

    fn active_token() -> usize {
        Cr3::read().0.start_address().as_u64() as usize
    }

    fn flush_tlb() {
        tlb::flush_all();
    }
}

impl Drop for PageTableImpl {
    fn drop(&mut self) {
        info!("PageTable dropping: {:?}", self.2);
        dealloc_frame(self.2.start_address().as_u64() as usize);
    }
}

struct FrameAllocatorForX86;

unsafe impl FrameAllocator<Size4KiB> for FrameAllocatorForX86 {
    fn allocate_frame(&mut self) -> Option<Frame> {
        alloc_frame().map(|addr| Frame::of_addr(addr))
    }
}

impl FrameDeallocator<Size4KiB> for FrameAllocatorForX86 {
    unsafe fn deallocate_frame(&mut self, frame: Frame) {
        dealloc_frame(frame.start_address().as_u64() as usize);
    }
}

/// Flush TLB for `vaddr` on all CPU
fn flush_tlb_all(_vaddr: usize) {
    // FIXME: too slow, disable now.
    return;
    // if !super::AP_CAN_INIT.load(Ordering::Relaxed) {
    //     return;
    // }
    // super::ipi::invoke_on_allcpu(move || tlb::flush(VirtAddr::new(vaddr as u64)), false);
}
