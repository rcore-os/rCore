use alloc::vec::Vec;
use super::*;
use core::fmt::{Debug, Formatter, Error};

/// 一片连续内存空间，有相同的访问权限
/// 对应ucore中 `vma_struct`
#[derive(Debug, Eq, PartialEq)]
pub struct MemoryArea {
    pub start_addr: VirtAddr,
    pub end_addr: VirtAddr,
    pub phys_start_addr: Option<PhysAddr>,
    pub flags: u32,
    pub name: &'static str,
    pub mapped: bool,
}

impl MemoryArea {
    pub fn contains(&self, addr: VirtAddr) -> bool {
        addr >= self.start_addr && addr < self.end_addr
    }
    fn is_overlap_with(&self, other: &MemoryArea) -> bool {
        let p0 = Page::of_addr(self.start_addr);
        let p1 = Page::of_addr(self.end_addr - 1) + 1;
        let p2 = Page::of_addr(other.start_addr);
        let p3 = Page::of_addr(other.end_addr - 1) + 1;
        !(p1 <= p2 || p0 >= p3)
    }
}

/// 内存空间集合，包含若干段连续空间
/// 对应ucore中 `mm_struct`
pub struct MemorySet {
    areas: Vec<MemoryArea>,
    page_table: Option<InactivePageTable>,
}

impl MemorySet {
    pub fn new() -> Self {
        MemorySet {
            areas: Vec::<MemoryArea>::new(),
            page_table: None,
        }
    }
    /// Used for remap_kernel() where heap alloc is unavailable
    pub unsafe fn new_from_raw_space(slice: &mut [u8]) -> Self {
        use core::mem::size_of;
        let cap = slice.len() / size_of::<MemoryArea>();
        MemorySet {
            areas: Vec::<MemoryArea>::from_raw_parts(slice.as_ptr() as *mut MemoryArea, 0, cap),
            page_table: None,
        }
    }
    pub fn find_area(&self, addr: VirtAddr) -> Option<&MemoryArea> {
        self.areas.iter().find(|area| area.contains(addr))
    }
    pub fn push(&mut self, area: MemoryArea) {
        assert!(area.start_addr <= area.end_addr, "invalid memory area");
        if let Some(phys_addr) = area.phys_start_addr {
            assert_eq!(area.start_addr % PAGE_SIZE, phys_addr.get() % PAGE_SIZE,
                       "virtual & physical start address must have same page offset");
        }
        assert!(self.areas.iter()
            .find(|other| area.is_overlap_with(other))
                    .is_none(), "memory area overlap");
        self.areas.push(area);
    }
    pub fn map(&mut self, pt: &mut Mapper) {
        for area in self.areas.iter_mut() {
            if area.mapped {
                continue
            }
            match area.phys_start_addr {
                Some(phys_start) => {
                    for page in Page::range_of(area.start_addr, area.end_addr) {
                        let frame = Frame::of_addr(phys_start.get() + page.start_address() - area.start_addr);
                        pt.map_to(page, frame.clone(), EntryFlags::from_bits(area.flags.into()).unwrap());
                    }
                },
                None => {
                    for page in Page::range_of(area.start_addr, area.end_addr) {
                        pt.map(page, EntryFlags::from_bits(area.flags.into()).unwrap());
                    }
                },
            }
            area.mapped = true;
        }
    }
    pub fn unmap(&mut self, pt: &mut Mapper) {
        for area in self.areas.iter_mut() {
            if !area.mapped {
                continue
            }
            for page in Page::range_of(area.start_addr, area.end_addr) {
                pt.unmap(page);
            }
            area.mapped = false;
        }
    }
}

impl Debug for MemorySet {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_list()
            .entries(self.areas.iter())
            .finish()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn push_and_find() {
        let mut ms = MemorySet::new();
        ms.push(MemoryArea {
            start_addr: 0x0,
            end_addr: 0x8,
            flags: 0x0,
            name: "code",
        });
        ms.push(MemoryArea {
            start_addr: 0x8,
            end_addr: 0x10,
            flags: 0x1,
            name: "data",
        });
        assert_eq!(ms.find_area(0x6).unwrap().name, "code");
        assert_eq!(ms.find_area(0x11), None);
    }

    #[test]
    #[should_panic]
    fn push_overlap() {
        let mut ms = MemorySet::new();
        ms.push(MemoryArea {
            start_addr: 0x0,
            end_addr: 0x8,
            flags: 0x0,
            name: "code",
        });
        ms.push(MemoryArea {
            start_addr: 0x4,
            end_addr: 0x10,
            flags: 0x1,
            name: "data",
        });
    }
}
