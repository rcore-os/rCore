use core::fmt;
use core::ops::{Index, IndexMut};

use super::{PageSize, PhysFrame, Size4KiB};
use addr::PhysAddr;

use usize_conversions::usize_from;
use ux::*;

/// The error returned by the `PageTableEntry::frame` method.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameError {
    /// The entry does not have the `PRESENT` flag set, so it isn't currently mapped to a frame.
    FrameNotPresent,
    /// The entry does have the `HUGE_PAGE` flag set. The `frame` method has a standard 4KiB frame
    /// as return type, so a huge frame can't be returned.
    HugeFrame,
}

/// A 64-bit page table entry.
#[derive(Clone)]
#[repr(transparent)]
pub struct PageTableEntry {
    pub entry: u64,
}

impl PageTableEntry {
    /// Returns whether this entry is zero.
    pub fn is_unused(&self) -> bool {
        self.entry == 0
    }

    /// Sets this entry to zero.
    pub fn set_unused(&mut self) {
        self.entry = 0;
    }

    /// Returns the flags of this entry.
    pub fn flags(&self) -> PageTableFlags {
        PageTableFlags::from_bits_truncate(self.entry)
    }

    /// Returns the physical address mapped by this entry, might be zero.
    pub fn addr(&self) -> PhysAddr {
        PhysAddr::new(self.entry & 0x0000_ffff_ffff_f000)
    }

    /// Returns the physical frame mapped by this entry.
    ///
    /// Returns the following errors:
    ///
    /// - `FrameError::FrameNotPresent` if the entry doesn't have the `PRESENT` flag set.
    /// - `FrameError::HugeFrame` if the entry has the `HUGE_PAGE` flag set (for huge pages the
    ///    `addr` function must be used)
    pub fn frame(&self) -> Result<PhysFrame, FrameError> {
        if !self.flags().contains(PageTableFlags::PRESENT) {
            Err(FrameError::FrameNotPresent)
        } else if self.flags().contains(PageTableFlags::HUGE_PAGE) {
            Err(FrameError::HugeFrame)
        } else {
            Ok(PhysFrame::containing_address(self.addr()))
        }
    }

    /// Map the entry to the specified physical address with the specified flags.
    pub fn set_addr(&mut self, addr: PhysAddr, flags: PageTableFlags) {
        assert!(addr.is_aligned(Size4KiB::SIZE));
        self.entry = (addr.as_u64()) | flags.bits();
    }

    /// Map the entry to the specified physical frame with the specified flags.
    pub fn set_frame(&mut self, frame: PhysFrame, flags: PageTableFlags) {
        assert!(!flags.contains(PageTableFlags::HUGE_PAGE));
        self.set_addr(frame.start_address(), flags)
    }

    /// Sets the flags of this entry.
    pub fn set_flags(&mut self, flags: PageTableFlags) {
        self.entry = self.addr().as_u64() | flags.bits();
    }
}

impl fmt::Debug for PageTableEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut f = f.debug_struct("PageTableEntry");
        f.field("value", &self.entry);
        f.field("addr", &self.addr());
        f.field("flags", &self.flags());
        f.finish()
    }
}

bitflags! {
    /// Possible flags for a page table entry.
    pub struct PageTableFlags: u64 {
        const ALL =             0xffffffff_ffffffff;
        const TYPE_MASK =       3 << 0;
        // const TYPE_FAULT =      0 << 0;
        const TYPE_PAGE =       3 << 0;
        const TABLE_BIT =       1 << 1;
        // const BLOCK_BIT =       0 << 1;
        const PAGE_BIT =        1 << 1;

        const PRESENT =         1 << 0;
        const USER_ACCESSIBLE = 1 << 6;         /* AP[1] */
        const RDONLY =          1 << 7;         /* AP[2] */
        const SHARED =          3 << 8;         /* SH[1:0], inner shareable */
        const BIT_8 =           1 << 8;
        const BIT_9 =           1 << 9;

        // pub const ATTRIB_SH_NON_SHAREABLE: usize = 0x0 << 8;
        const OUTER_SHAREABLE = 0b10 << 8;
        const INNER_SHAREABLE = 0b11 << 8;

        const ACCESSED =        1 << 10;        /* AF, Access Flag */
        const NONE_GLOBAL =     1 << 11;        /* None Global */
        const GLOBAL =          (!(1 << 11));
        const DBM =             1 << 51;        /* Dirty Bit Management */
        const WRITE =           1 << 51;        /* DBM */
        const CONT =            1 << 52;        /* Contiguous range */
        const PXN =             1 << 53;        /* Privileged XN */
        const UXN =             1 << 54;        /* User XN */
        const HYP_XN =          1 << 54;        /* HYP XN */
        const DIRTY =           1 << 55;
        const SWAPPED =         1 << 56;
        const HUGE_PAGE =       1 << 57;
        const PROT_NONE =       1 << 58;

    }
}

/// The number of entries in a page table.
const ENTRY_COUNT: usize = 512;

/// Represents a page table.
///
/// Always page-sized.
///
/// This struct implements the `Index` and `IndexMut` traits, so the entries can be accessed
/// through index operations. For example, `page_table[15]` returns the 15th page table entry.
#[repr(transparent)]
pub struct PageTable {
    entries: [PageTableEntry; ENTRY_COUNT],
}

impl PageTable {
    /// Clears all entries.
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.set_unused();
        }
    }

    /// Setup identity map: VirtPage at pagenumber -> PhysFrame at pagenumber
    /// pn: pagenumber = addr>>12 in riscv32.
    pub fn map_identity(&mut self, p4num: usize, flags: PageTableFlags) {
        let entry = self.entries[p4num].clone();
        self.entries[p4num].set_addr(entry.addr(), flags);
    }
}

impl Index<usize> for PageTable {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

impl Index<u9> for PageTable {
    type Output = PageTableEntry;

    fn index(&self, index: u9) -> &Self::Output {
        &self.entries[usize_from(u16::from(index))]
    }
}

impl IndexMut<u9> for PageTable {
    fn index_mut(&mut self, index: u9) -> &mut Self::Output {
        &mut self.entries[usize_from(u16::from(index))]
    }
}

impl fmt::Debug for PageTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.entries[..].fmt(f)
    }
}
