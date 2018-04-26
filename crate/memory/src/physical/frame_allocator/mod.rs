use super::*;

pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame>;
    fn deallocate_frame(&mut self, frame: Frame);
}

pub trait MemoryArea {
    fn begin(&self) -> PhysAddr;
    fn end(&self) -> PhysAddr;
}