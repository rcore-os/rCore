// Depends on kernel
use crate::memory::{active_table, alloc_frame, dealloc_frame};
use rcore_memory::paging::*;
use x86_64::instructions::tlb;
use x86_64::PhysAddr;
use x86_64::registers::control::{Cr3, Cr3Flags};
use x86_64::structures::paging::{Mapper, PageTable as x86PageTable, PageTableEntry, PageTableFlags as EF, RecursivePageTable};
use x86_64::structures::paging::{FrameAllocator, FrameDeallocator, Page, PageRange, PhysFrame as Frame, Size4KiB};
use log::*;

pub trait PageExt {
    fn of_addr(address: usize) -> Self;
    fn range_of(begin: usize, end: usize) -> PageRange;
}

impl PageExt for Page {
    fn of_addr(address: usize) -> Self {
        use x86_64;
        Page::containing_address(x86_64::VirtAddr::new(address as u64))
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

pub struct ActivePageTable(RecursivePageTable<'static>);

pub struct PageEntry(PageTableEntry);

impl PageTable for ActivePageTable {
    fn map(&mut self, addr: usize, target: usize) -> &mut Entry {
        let flags = EF::PRESENT | EF::WRITABLE | EF::NO_EXECUTE;
        unsafe {
            self.0.map_to(Page::of_addr(addr), Frame::of_addr(target), flags, &mut FrameAllocatorForX86)
                .unwrap().flush();
        }
        unsafe { &mut *(get_entry_ptr(addr, 1)) }
    }

    fn unmap(&mut self, addr: usize) {
        let (_, flush) = self.0.unmap(Page::of_addr(addr)).unwrap();
        flush.flush();
    }

    fn get_entry(&mut self, addr: usize) -> Option<&mut Entry> {
        for level in 0..3 {
            let entry = get_entry_ptr(addr, 4 - level);
            if unsafe { !(*entry).present() } { return None; }
        }
        unsafe { Some(&mut *(get_entry_ptr(addr, 1))) }
    }
}

impl PageTableExt for ActivePageTable {}

impl ActivePageTable {
    pub unsafe fn new() -> Self {
        ActivePageTable(RecursivePageTable::new(&mut *(0xffffffff_fffff000 as *mut _)).unwrap())
    }
}

impl Entry for PageEntry {
    fn update(&mut self) {
        use x86_64::{VirtAddr, instructions::tlb::flush};
        let addr = VirtAddr::new_unchecked((self as *const _ as u64) << 9);
        flush(addr);
    }
    fn accessed(&self) -> bool { self.0.flags().contains(EF::ACCESSED) }
    fn dirty(&self) -> bool { self.0.flags().contains(EF::DIRTY) }
    fn writable(&self) -> bool { self.0.flags().contains(EF::WRITABLE) }
    fn present(&self) -> bool { self.0.flags().contains(EF::PRESENT) }
    fn clear_accessed(&mut self) { self.as_flags().remove(EF::ACCESSED); }
    fn clear_dirty(&mut self) { self.as_flags().remove(EF::DIRTY); }
    fn set_writable(&mut self, value: bool) { self.as_flags().set(EF::WRITABLE, value); }
    fn set_present(&mut self, value: bool) { self.as_flags().set(EF::PRESENT, value); }
    fn target(&self) -> usize { self.0.addr().as_u64() as usize }
    fn set_target(&mut self, target: usize) {
        let flags = self.0.flags();
        self.0.set_addr(PhysAddr::new(target as u64), flags);
    }
    fn writable_shared(&self) -> bool { self.0.flags().contains(EF::BIT_10) }
    fn readonly_shared(&self) -> bool { self.0.flags().contains(EF::BIT_9) }
    fn set_shared(&mut self, writable: bool) {
        let flags = self.as_flags();
        flags.set(EF::BIT_10, writable);
        flags.set(EF::BIT_9, !writable);
    }
    fn clear_shared(&mut self) { self.as_flags().remove(EF::BIT_9 | EF::BIT_10); }
    fn swapped(&self) -> bool { self.0.flags().contains(EF::BIT_11) }
    fn set_swapped(&mut self, value: bool) { self.as_flags().set(EF::BIT_11, value); }
    fn user(&self) -> bool { self.0.flags().contains(EF::USER_ACCESSIBLE) }
    fn set_user(&mut self, value: bool) {
        self.as_flags().set(EF::USER_ACCESSIBLE, value);
        if value {
            let mut addr = self as *const _ as usize;
            for _ in 0..3 {
                // Upper level entry
                addr = ((addr >> 9) & 0o777_777_777_7770) | 0xffffff80_00000000;
                // set USER_ACCESSIBLE
                unsafe { (*(addr as *mut EF)).insert(EF::USER_ACCESSIBLE) };
            }
        }
    }
    fn execute(&self) -> bool { !self.0.flags().contains(EF::NO_EXECUTE) }
    fn set_execute(&mut self, value: bool) { self.as_flags().set(EF::NO_EXECUTE, !value); }
    fn mmio(&self) -> u8 { 0 }
    fn set_mmio(&mut self, _value: u8) { }
}

fn get_entry_ptr(addr: usize, level: u8) -> *mut PageEntry {
    debug_assert!(level <= 4);
    let entry_addr = ((addr >> (level * 9)) & !0x7) | !((1 << (48 - level * 9)) - 1);
    entry_addr as *mut PageEntry
}

impl PageEntry {
    fn as_flags(&mut self) -> &mut EF {
        unsafe { &mut *(self as *mut _ as *mut EF) }
    }
}

#[derive(Debug)]
pub struct InactivePageTable0 {
    p4_frame: Frame,
}

impl InactivePageTable for InactivePageTable0 {
    type Active = ActivePageTable;

    fn new_bare() -> Self {
        let target = alloc_frame().expect("failed to allocate frame");
        let frame = Frame::of_addr(target);
        active_table().with_temporary_map(target, |_, table: &mut x86PageTable| {
            table.zero();
            // set up recursive mapping for the table
            table[511].set_frame(frame.clone(), EF::PRESENT | EF::WRITABLE);
        });
        InactivePageTable0 { p4_frame: frame }
    }

    fn map_kernel(&mut self) {
        let table = unsafe { &mut *(0xffffffff_fffff000 as *mut x86PageTable) };
        // Kernel at 0xffff_ff00_0000_0000
        // Kernel stack at 0x0000_57ac_0000_0000 (defined in bootloader crate)
        let e510 = table[510].clone();
        let estack = table[175].clone();
        self.edit(|_| {
            table[510].set_addr(e510.addr(), e510.flags() | EF::GLOBAL);
            table[175].set_addr(estack.addr(), estack.flags() | EF::GLOBAL);
        });
    }

    fn token(&self) -> usize {
        self.p4_frame.start_address().as_u64() as usize // as CR3
    }

    unsafe fn set_token(token: usize) {
        Cr3::write(Frame::containing_address(PhysAddr::new(token as u64)), Cr3Flags::empty());
    }

    fn active_token() -> usize {
        Cr3::read().0.start_address().as_u64() as usize
    }

    fn flush_tlb() {
        tlb::flush_all();
    }

    fn edit<T>(&mut self, f: impl FnOnce(&mut Self::Active) -> T) -> T {
        let target = Cr3::read().0.start_address().as_u64() as usize;
        active_table().with_temporary_map(target, |active_table, p4_table: &mut x86PageTable| {
            let backup = p4_table[0o777].clone();

            // overwrite recursive mapping
            p4_table[0o777].set_frame(self.p4_frame.clone(), EF::PRESENT | EF::WRITABLE);
            tlb::flush_all();

            // execute f in the new context
            let ret = f(active_table);

            // restore recursive mapping to original p4 table
            p4_table[0o777] = backup;
            tlb::flush_all();
            ret
        })
    }
}

impl Drop for InactivePageTable0 {
    fn drop(&mut self) {
        info!("PageTable dropping: {:?}", self);
        dealloc_frame(self.p4_frame.start_address().as_u64() as usize);
    }
}

struct FrameAllocatorForX86;

impl FrameAllocator<Size4KiB> for FrameAllocatorForX86 {
    fn allocate_frame(&mut self) -> Option<Frame> {
        alloc_frame().map(|addr| Frame::of_addr(addr))
    }
}

impl FrameDeallocator<Size4KiB> for FrameAllocatorForX86 {
    fn deallocate_frame(&mut self, frame: Frame) {
        dealloc_frame(frame.start_address().as_u64() as usize);
    }
}
