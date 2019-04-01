//! Implememnt the swap manager with the FIFO page replacement algorithm

use super::*;
use alloc::collections::VecDeque;

#[derive(Default)]
pub struct FifoSwapManager {
    deque: VecDeque<Frame>,
}

impl SwapManager for FifoSwapManager {
    fn tick(&mut self) {}

    fn push(&mut self, frame: Frame) {
        info!(
            "SwapManager push token: {:x?} vaddr: {:x?}",
            frame.get_token(),
            frame.get_virtaddr()
        );
        self.deque.push_back(frame);
    }

    fn remove(&mut self, token: usize, addr: VirtAddr) {
        info!("SwapManager remove token: {:x?} vaddr: {:x?}", token, addr);
        let id = self
            .deque
            .iter()
            .position(|ref x| x.get_virtaddr() == addr && x.get_token() == token)
            .expect("address not found");
        self.deque.remove(id);
        //info!("SwapManager remove token finished: {:x?} vaddr: {:x?}", token, addr);
    }

    fn pop<T, S>(&mut self, _: &mut T, _: &mut S) -> Option<Frame>
    where
        T: PageTable,
        S: Swapper,
    {
        self.deque.pop_front()
    }
}

/*
#[cfg(test)]
mod test {
    use super::*;
    use swap::test::*;

    #[test]
    fn test() {
        use self::MemOp::{R, W};
        let ops = [
            R(0x1000), R(0x2000), R(0x3000), R(0x4000),
            W(0x3000), W(0x1000), W(0x4000), W(0x2000), W(0x5000),
            W(0x2000), W(0x1000), W(0x2000), W(0x3000), W(0x4000),
            W(0x5000), R(0x1000), W(0x1000)];
        let pgfault_count = [
            1, 2, 3, 4,
            4, 4, 4, 4, 5,
            5, 6, 7, 8, 9,
            10, 11, 11];
        test_manager(FifoSwapManager::default(), &ops, &pgfault_count);
    }
}
*/
