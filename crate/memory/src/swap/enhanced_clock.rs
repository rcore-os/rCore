//! Implememnt the swap manager with the enhanced clock page replacement algorithm

use alloc::collections::VecDeque;
use super::*;
use crate::paging::Entry;

#[derive(Default)]
pub struct EnhancedClockSwapManager {
    clock_ptr: usize,
    deque: VecDeque<VirtAddr>,
}

// FIXME: It's unusable. But can pass a simple test.
impl SwapManager for EnhancedClockSwapManager {
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

    fn pop<T, S>(&mut self, page_table: &mut T, _swapper: &mut S) -> Option<VirtAddr>
        where T: PageTable, S: Swapper
    {
        loop {
            let addr = self.deque[self.clock_ptr];
            // FIXME: Once define `slice`, all modifies of `entry` below will fail.
            //        Then the loop will never stop.
            //        The reason may be `get_page_slice_mut()` contains unsafe operation,
            //        which lead the compiler to do a wrong optimization.
//            let slice = page_table.get_page_slice_mut(addr);
            let entry = page_table.get_entry(addr).unwrap();
//            println!("{:#x} , {}, {}", addr, entry.accessed(), entry.dirty());

            match (entry.accessed(), entry.dirty()) {
                (true, _) => {
                    entry.clear_accessed();
                },
                (false, true) => {
                    // FIXME:
//                    if let Ok(token) = swapper.swap_out(slice) {
//                        entry.clear_dirty();
//                    }
                },
                _ => {
                    return self.remove_current();
                }
            }
            self.move_next();
        }
    }
}


impl EnhancedClockSwapManager {
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
    use swap::test::*;

    #[test]
    fn test() {
        use self::MemOp::{R, W};
        let ops = [
            R(0x1000), R(0x2000), R(0x3000), R(0x4000),
            R(0x3000), W(0x1000), R(0x4000), W(0x2000), R(0x5000),
            R(0x2000), W(0x1000), R(0x2000), R(0x3000), R(0x4000)];
        let pgfault_count = [
            1, 2, 3, 4,
            4, 4, 4, 4, 5,
            5, 5, 5, 6, 7];
        test_manager(EnhancedClockSwapManager::default(), &ops, &pgfault_count);
    }
}
