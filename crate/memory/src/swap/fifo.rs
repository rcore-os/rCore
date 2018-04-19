use alloc::vec_deque::VecDeque;
use super::*;

struct FifoSwapManager<T: 'static + PageTable> {
    page_table: &'static T,
    deque: VecDeque<Addr>,
}

impl<T: 'static + PageTable> SwapManager<T> for FifoSwapManager<T> {
    fn new(page_table: &'static T) -> Self {
        FifoSwapManager {
            page_table,
            deque: VecDeque::<Addr>::new()
        }
    }

    fn tick(&mut self) {

    }

    fn push(&mut self, addr: usize) {
        self.deque.push_back(addr);
    }

    fn pop(&mut self, addr: usize) {
        let id = self.deque.iter()
            .position(|&x| x == addr)
            .expect("address not found");
        self.deque.remove(id);
    }

    fn swap(&mut self) -> Option<Addr> {
        self.deque.pop_front()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::mock_page_table::MockPageTable;

    enum MemOp {
        R(usize), W(usize)
    }

    fn assert_pgfault_eq(x: usize) {
        assert_eq!(unsafe{ PGFAULT_COUNT }, x);
    }

    // For pgfault_handler:
    static mut PGFAULT_COUNT: usize = 0;
    static mut PAGE: *mut MockPageTable = 0 as *mut _;
    static mut FIFO: *mut FifoSwapManager<MockPageTable> = 0 as *mut _;

    fn pgfault_handler(pt: &mut MockPageTable, addr: Addr) {
        unsafe{ PGFAULT_COUNT += 1; }
        let fifo = unsafe{ &mut *FIFO };
        if let Some(x) = fifo.swap() {
            pt.unmap(x);
        }
        pt.map(addr);
    }

    #[test]
    fn test() {
        use self::MemOp::{R, W};
        let mut pt = MockPageTable::new(pgfault_handler);
        let mut fifo = FifoSwapManager::<MockPageTable>::new(
            unsafe{ &*(&pt as *const _) });
        unsafe {
            PAGE = &mut pt as *mut _;
            FIFO = &mut fifo as *mut _;
        }
        let op_seq = [
            R(1), R(2), R(3), R(4),
            W(3), W(1), W(4), W(2), W(5),
            W(2), W(1), W(2), W(3), W(4),
            W(5), R(1), W(1)];
        let pgfault_count = [
            1, 2, 3, 4,
            4, 4, 4, 4, 5,
            5, 6, 7, 8, 9,
            10, 11, 11];
        for (op, &count) in op_seq.iter().zip(pgfault_count.iter()) {
            match op {
                R(addr) => pt.read(*addr),
                W(addr) => pt.write(*addr),
            }
            assert_pgfault_eq(count);
        }
    }
}