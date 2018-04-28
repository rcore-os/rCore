use alloc::vec::Vec;
use super::*;

/// 一片连续内存空间，有相同的访问权限
/// 对应ucore中 `vma_struct`
#[derive(Debug, Eq, PartialEq)]
pub struct MemoryArea {
    pub start_addr: VirtAddr,
    pub end_addr: VirtAddr,
    pub flags: u32,
    pub name: &'static str,
    pub mapped: bool,
}

impl MemoryArea {
    pub fn contains(&self, addr: VirtAddr) -> bool {
        addr >= self.start_addr && addr < self.end_addr
    }
    fn is_overlap_with(&self, other: &MemoryArea) -> bool {
        !(self.end_addr <= other.start_addr || self.start_addr >= other.end_addr)
    }
}

/// 内存空间集合，包含若干段连续空间
/// 对应ucore中 `mm_struct`
pub struct MemorySet {
    areas: Vec<MemoryArea>,
    page_table: InactivePageTable,
}

impl MemorySet {
    pub fn new(mc: &mut MemoryController) -> Self {
        MemorySet {
            areas: Vec::<MemoryArea>::new(),
            page_table: mc.new_page_table(),
        }
    }
    pub fn find_area(&self, addr: VirtAddr) -> Option<&MemoryArea> {
        self.areas.iter().find(|area| area.contains(addr))
    }
    pub fn push(&mut self, area: MemoryArea) {
        debug_assert!(area.start_addr <= area.end_addr, "invalid memory area");
        if self.areas.iter()
            .find(|other| area.is_overlap_with(other))
            .is_some() {
            panic!("memory area overlap");
        }
        self.areas.push(area);
    }
    pub fn map(&mut self, mc: &mut MemoryController) {
//        mc.active_table.with(self.page_table, )
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
