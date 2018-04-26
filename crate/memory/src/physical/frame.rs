use super::*;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    number: usize,
}

impl Frame {
    pub fn containing_address(address: PhysAddr) -> Frame {
        Frame{ number: address.get() as usize / PAGE_SIZE }
    }
    //TODO: Set private
    pub fn start_address(&self) -> PhysAddr {
        PhysAddr::new((self.number * PAGE_SIZE) as u64)
    }

    pub fn clone(&self) -> Frame {
        Frame { number: self.number }
    }
    //TODO: Set private
//    pub fn range_inclusive(start: Frame, end: Frame) -> FrameIter {
//        FrameIter {
//            start: start,
//            end: end,
//        }
//    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        panic!("frame must be deallocate");
    }
}
