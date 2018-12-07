use super::*;

// here may be a interesting part for lab
pub trait MemoryHandler: Debug + 'static {
    fn box_clone(&self) -> Box<MemoryHandler>;
    fn map(&self, pt: &mut PageTable, addr: VirtAddr);
    fn unmap(&self, pt: &mut PageTable, addr: VirtAddr);
    fn page_fault_handler(&self, pt: &mut PageTable, addr: VirtAddr) -> bool;
}

impl Clone for Box<MemoryHandler> {
    fn clone(&self) -> Box<MemoryHandler> {
        self.box_clone()
    }
}

pub trait FrameAllocator: Debug + Clone + 'static {
    fn alloc(&self) -> Option<PhysAddr>;
    fn dealloc(&self, target: PhysAddr);
}

mod linear;
mod byframe;
mod delay;
//mod swap;

pub use self::linear::Linear;
pub use self::byframe::ByFrame;
pub use self::delay::Delay;