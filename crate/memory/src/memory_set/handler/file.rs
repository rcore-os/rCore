use super::*;

/// Delay mapping a page to an area of a file.
#[derive(Clone)]
pub struct File<F, T> {
    pub file: F,
    pub mem_start: usize,
    pub file_start: usize,
    pub file_end: usize,
    pub allocator: T,
}

pub trait Read: Clone + Send + Sync + 'static {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize;
}

impl<F: Read, T: FrameAllocator> MemoryHandler for File<F, T> {
    fn box_clone(&self) -> Box<dyn MemoryHandler> {
        Box::new(self.clone())
    }

    fn map(&self, pt: &mut dyn PageTable, addr: usize, attr: &MemoryAttr) {
        let entry = pt.map(addr, 0);
        entry.set_present(false);
        attr.apply(entry);
    }

    fn unmap(&self, pt: &mut dyn PageTable, addr: usize) {
        let entry = pt.get_entry(addr).expect("failed to get entry");
        if entry.present() {
            self.allocator.dealloc(entry.target());
        }

        // PageTable::unmap requires page to be present
        entry.set_present(true);
        pt.unmap(addr);
    }

    fn clone_map(
        &self,
        pt: &mut dyn PageTable,
        src_pt: &mut dyn PageTable,
        addr: usize,
        attr: &MemoryAttr,
    ) {
        let entry = src_pt.get_entry(addr).expect("failed to get entry");
        if entry.present() && !attr.readonly {
            // eager map and copy data
            let data = src_pt.get_page_slice_mut(addr);
            let target = self.allocator.alloc().expect("failed to alloc frame");
            let entry = pt.map(addr, target);
            attr.apply(entry);
            pt.get_page_slice_mut(addr).copy_from_slice(data);
            pt.flush_cache_copy_user(addr, addr + data.len(), attr.execute);
        } else {
            // delay map
            self.map(pt, addr, attr);
        }
    }

    fn handle_page_fault(&self, pt: &mut dyn PageTable, addr: usize) -> bool {
        let addr = addr & !(PAGE_SIZE - 1);
        let entry = pt.get_entry(addr).expect("failed to get entry");
        if entry.present() {
            return false;
        }
        let execute = entry.execute();
        let frame = self.allocator.alloc().expect("failed to alloc frame");
        entry.set_target(frame);
        entry.set_present(true);
        entry.update();

        let read_size = self.fill_data(pt, addr);
        pt.flush_cache_copy_user(addr, addr + read_size, execute);
        true
    }
}

impl<F: Read, T: FrameAllocator> File<F, T> {
    fn fill_data(&self, pt: &mut dyn PageTable, addr: VirtAddr) -> usize {
        let data = pt.get_page_slice_mut(addr);
        let file_offset = addr + self.file_start - self.mem_start;
        let read_size = (self.file_end as isize - file_offset as isize)
            .min(PAGE_SIZE as isize)
            .max(0) as usize;
        let read_size = self.file.read_at(file_offset, &mut data[..read_size]);
        if read_size != PAGE_SIZE {
            data[read_size..].iter_mut().for_each(|x| *x = 0);
        }
        read_size
    }
}

impl<F, T> Debug for File<F, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.debug_struct("FileHandler")
            .field("mem_start", &self.mem_start)
            .field("file_start", &self.file_start)
            .field("file_end", &self.file_end)
            .finish()
    }
}
