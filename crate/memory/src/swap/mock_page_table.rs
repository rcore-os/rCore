use alloc::btree_set::BTreeSet;
use super::*;

pub struct MockPageTable {
    mapped_set: BTreeSet<Addr>,
    accessed_set: BTreeSet<Addr>,
    dirty_set: BTreeSet<Addr>,
    pgfault_handler: PgfaultHandler,
}

type PgfaultHandler = fn(&mut MockPageTable, Addr);

impl PageTable for MockPageTable {
    fn accessed(&self, addr: Addr) -> bool {
        self.accessed_set.contains(&addr)
    }
    fn dirty(&self, addr: Addr) -> bool {
        self.dirty_set.contains(&addr)
    }
}

impl MockPageTable {
    pub fn new(pgfault_handler: PgfaultHandler) -> Self {
        MockPageTable {
            mapped_set: BTreeSet::<Addr>::new(),
            accessed_set: BTreeSet::<Addr>::new(),
            dirty_set: BTreeSet::<Addr>::new(),
            pgfault_handler,
        }
    }
    /// Read memory, mark accessed, trigger page fault if not present
    pub fn read(&mut self, addr: Addr) {
        while !self.mapped_set.contains(&addr) {
            (self.pgfault_handler)(self, addr);
        }
        self.accessed_set.insert(addr);

    }
    /// Write memory, mark accessed and dirty, trigger page fault if not present
    pub fn write(&mut self, addr: Addr) {
        while !self.mapped_set.contains(&addr) {
            (self.pgfault_handler)(self, addr);
        }
        self.accessed_set.insert(addr);
        self.dirty_set.insert(addr);
    }
    pub fn map(&mut self, addr: Addr) {
        self.mapped_set.insert(addr);
    }
    pub fn unmap(&mut self, addr: Addr) {
        self.mapped_set.remove(&addr);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    static mut PGFAULT_COUNT: usize = 0;

    fn assert_pgfault_eq(x: usize) {
        assert_eq!(unsafe{ PGFAULT_COUNT }, x);
    }

    #[test]
    fn test() {
        fn pgfault_handler(pt: &mut MockPageTable, addr: Addr) {
            unsafe{ PGFAULT_COUNT += 1; }
            pt.map(addr);
        }
        let mut pt = MockPageTable::new(pgfault_handler);

        pt.map(0);
        pt.read(0);
        assert_pgfault_eq(0);
        assert!(pt.accessed(0));
        assert!(!pt.dirty(0));

        pt.write(1);
        assert_pgfault_eq(1);
        assert!(pt.accessed(1));
        assert!(pt.dirty(1));

        pt.unmap(0);
        pt.read(0);
        assert_pgfault_eq(2);
    }
}