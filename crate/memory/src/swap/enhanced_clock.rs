use alloc::vec_deque::VecDeque;
use super::*;

pub struct EnhancedClockSwapManager<T: 'static + SwappablePageTable> {
    page_table: &'static mut T,
    clock_ptr: usize,
    deque: VecDeque<VirtAddr>,
}

impl<T: 'static + SwappablePageTable> SwapManager for EnhancedClockSwapManager<T> {
    fn tick(&mut self) {
    }

    fn push(&mut self, addr: usize) {
        let pos = if self.clock_ptr == 0 {self.deque.len()} else {self.clock_ptr};
        self.deque.insert(pos, addr);
    }

    fn remove(&mut self, addr: usize) {
        let id = self.deque.iter()
            .position(|&x| x == addr)
            .expect("address not found");
        if id < self.clock_ptr {
            self.clock_ptr -= 1;
        }
        self.deque.remove(id);
    }

    fn pop(&mut self) -> Option<usize> {
        loop {
            let addr = self.deque[self.clock_ptr];
            let accessed = self.page_table.accessed(addr);
            let dirty = self.page_table.dirty(addr);

            match (accessed, dirty) {
                (true, _) => {
                    self.page_table.clear_accessed(addr);

                },
                (false, true) => {
                    if self.page_table.swap_out(addr).is_ok() {
                        self.page_table.clear_dirty(addr);
                    }
                },
                _ => {
                    return self.remove_current();
                }
            }
            self.move_next();
        }
    }
}


impl<T: 'static + SwappablePageTable> EnhancedClockSwapManager<T> {
    pub fn new(page_table: &'static mut T) -> Self {
        EnhancedClockSwapManager {
            page_table,
            clock_ptr: 0,
            deque: VecDeque::<VirtAddr>::new()
        }
    }
    fn remove_current(&mut self) -> Option<VirtAddr> {
        let addr = self.deque.remove(self.clock_ptr);
        if self.clock_ptr == self.deque.len() {
            self.clock_ptr = 0;
        }
        return addr;
    }
    fn move_next(&mut self) {
        self.clock_ptr += 1;
        if self.clock_ptr == self.deque.len() {
            self.clock_ptr = 0;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::{arc::Arc, boxed::Box};
    use core::mem::uninitialized;
    use core::cell::RefCell;
    use page_table::mock_page_table::MockPageTable;

    impl SwappablePageTable for MockPageTable {
        fn swap_out(&mut self, addr: usize) -> Result<(), ()> {
            Ok(())
        }
    }

    enum MemOp {
        R(usize), W(usize)
    }

    #[test]
    fn test() {
        use self::MemOp::{R, W};
        let page_fault_count = Arc::new(RefCell::new(0usize));

        let mut pt = Box::new(MockPageTable::new(4));
        let static_pt = unsafe{ &mut *(pt.as_mut() as *mut MockPageTable) };
        pt.set_handler(Box::new({
            let page_fault_count1 = page_fault_count.clone();
            let mut clock = EnhancedClockSwapManager::new(static_pt);

            move |pt: &mut MockPageTable, addr: VirtAddr| {
                *page_fault_count1.borrow_mut() += 1;

                if !pt.map(addr) {  // is full?
                    pt.unmap(clock.pop().unwrap());
                    pt.map(addr);
                }
                clock.push(addr);
            }
        }));


        let op_seq = [
            R(1), R(2), R(3), R(4),
            R(3), W(1), R(4), W(2), R(5),
            R(2), W(1), R(2), R(3), R(4)];
        let pgfault_count = [
            1, 2, 3, 4,
            4, 4, 4, 4, 5,
            5, 5, 5, 6, 7];
        for (op, &count) in op_seq.iter().zip(pgfault_count.iter()) {
            match op {
                R(addr) => pt.read(*addr),
                W(addr) => pt.write(*addr),
            }
            assert_eq!(*(*page_fault_count).borrow(), count);
        }
    }
}
