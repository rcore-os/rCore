use alloc::vec_deque::VecDeque;
use super::*;

struct FifoSwapManager {
    deque: VecDeque<Addr>,
}

impl<T: PageTable> SwapManager<T> for FifoSwapManager {
    fn new(_page_table: &T) -> Self {
        FifoSwapManager {
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

    #[test]
    fn test() {
//        let mut pt = mock_page_table::MockPageTable::new();
//        let mut sm = FifoSwapManager::new();
//        let write_seq = [3, 1, 4, 2, 5, 2, 1, 2, 3, 4, 5, 1, 1];
//        let pgfault_count = [4, 4, 4, 4, 5, 5, 6, 7, 8, 9, 10, 11, 11];
//        for i in write_seq {
//            pt.write(i);
//        }

    }
}