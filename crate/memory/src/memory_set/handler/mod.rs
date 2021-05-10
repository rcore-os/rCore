use super::*;
#[derive(Copy, Clone, Debug)]
pub struct AccessType {
    pub write: bool,
    pub execute: bool,
    pub user: bool,
}
impl AccessType {
    pub fn unknown() -> Self {
        AccessType {
            write: true,
            execute: true,
            user: true,
        }
    }
    pub fn read(user: bool) -> Self {
        AccessType {
            write: false,
            execute: false,
            user,
        }
    }
    pub fn write(user: bool) -> Self {
        AccessType {
            write: true,
            execute: false,
            user,
        }
    }
    pub fn execute(user: bool) -> Self {
        AccessType {
            write: false,
            execute: true,
            user,
        }
    }
    pub fn check_access(self, entry: &dyn paging::Entry) -> bool {
        ((!self.write) || entry.writable())
            && ((!self.execute) || entry.execute())
            && ((!self.user) || entry.user())
    }
}
// here may be a interesting part for lab
pub trait MemoryHandler: Debug + Send + Sync + 'static {
    fn box_clone(&self) -> Box<dyn MemoryHandler>;

    /// Map `addr` in the page table
    /// Should set page flags here instead of in `page_fault_handler`
    fn map(&self, pt: &mut dyn PageTable, addr: VirtAddr, attr: &MemoryAttr);

    /// Unmap `addr` in the page table
    fn unmap(&self, pt: &mut dyn PageTable, addr: VirtAddr);

    /// Clone map `addr` from page table `src_pt` to `pt`.
    fn clone_map(
        &self,
        pt: &mut dyn PageTable,
        src_pt: &mut dyn PageTable,
        addr: VirtAddr,
        attr: &MemoryAttr,
    );

    /// Handle page fault on `addr`
    /// Return true if success, false if error
    fn handle_page_fault(&self, pt: &mut dyn PageTable, addr: VirtAddr) -> bool {
        self.handle_page_fault_ext(pt, addr, AccessType::unknown())
    }

    /// Handle page fault on `addr` and access type `access`
    /// Return true if success (or should-retry), false if error
    fn handle_page_fault_ext(
        &self,
        pt: &mut dyn PageTable,
        addr: VirtAddr,
        _access: AccessType,
    ) -> bool {
        self.handle_page_fault(pt, addr)
    }
}

impl Clone for Box<dyn MemoryHandler> {
    fn clone(&self) -> Box<dyn MemoryHandler> {
        self.box_clone()
    }
}

pub trait FrameAllocator: Debug + Clone + Send + Sync + 'static {
    fn alloc(&self) -> Option<PhysAddr>;
    fn alloc_contiguous(&self, size: usize, align_log2: usize) -> Option<PhysAddr>;
    fn dealloc(&self, target: PhysAddr);
}

mod byframe;
mod delay;
mod file;
mod linear;
mod shared;
//mod swap;

pub use self::byframe::ByFrame;
pub use self::delay::Delay;
pub use self::file::{File, Read};
pub use self::linear::Linear;
pub use self::shared::{Shared, SharedGuard};
