use super::*;

// here may be a interesting part for lab
pub trait MemoryHandler: Debug + 'static {
    fn box_clone(&self) -> Box<MemoryHandler>;

    /// Map `addr` in the page table
    /// Should set page flags here instead of in page_fault_handler
    fn map(&self, pt: &mut PageTable, addr: VirtAddr, attr: &MemoryAttr);
    
    /// Map `addr` in the page table eagerly (i.e. no delay allocation)
    /// Should set page flags here instead of in page_fault_handler
    fn map_eager(&self, pt: &mut PageTable, addr: VirtAddr, attr: &MemoryAttr) {
        // override this when pages are allocated lazily
        self.map(pt, addr, attr);
    }

    /// Unmap `addr` in the page table
    fn unmap(&self, pt: &mut PageTable, addr: VirtAddr);

    /// Handle page fault on `addr`
    /// Return true if success, false if error
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
