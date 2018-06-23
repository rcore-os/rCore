pub use arch::paging::*;
use bit_allocator::{BitAlloc, BitAlloc64K};
use consts::KERNEL_OFFSET;
use multiboot2::{ElfSection, ElfSectionFlags, ElfSectionsTag};
use multiboot2::BootInformation;
pub use self::address::*;
pub use self::memory_set::*;
pub use self::stack_allocator::*;
use spin::{Mutex, MutexGuard};
use super::HEAP_ALLOCATOR;
use ucore_memory::paging::PageTable;

mod memory_set;
mod stack_allocator;
mod address;

const PAGE_SIZE: usize = 1 << 12;

lazy_static! {
    static ref FRAME_ALLOCATOR: Mutex<BitAlloc64K> = Mutex::new(BitAlloc64K::default());
}
static STACK_ALLOCATOR: Mutex<Option<StackAllocator>> = Mutex::new(None);

pub fn alloc_frame() -> Frame {
    let frame = frame_allocator().alloc().expect("no more frame");
    trace!("alloc: {:?}", frame);
    frame
}

pub fn dealloc_frame(frame: Frame) {
    trace!("dealloc: {:?}", frame);
    frame_allocator().dealloc(frame);
}

fn alloc_stack(size_in_pages: usize) -> Stack {
    let mut active_table = active_table();
    STACK_ALLOCATOR.lock()
        .as_mut().expect("stack allocator is not initialized")
        .alloc_stack(&mut active_table, size_in_pages).expect("no more stack")
}

lazy_static! {
    static ref ACTIVE_TABLE: Mutex<CowExt<ActivePageTable>> = Mutex::new(unsafe {
        CowExt::new(ActivePageTable::new())
    });
}

/// The only way to get active page table
fn active_table() -> MutexGuard<'static, CowExt<ActivePageTable>> {
    ACTIVE_TABLE.lock()
}

pub fn frame_allocator() -> BitAllocGuard {
    BitAllocGuard(FRAME_ALLOCATOR.lock())
}

// Return true to continue, false to halt
pub fn page_fault_handler(addr: VirtAddr) -> bool {
    // Handle copy on write
    unsafe { ACTIVE_TABLE.force_unlock(); }
    active_table().page_fault_handler(addr, || alloc_frame().start_address().as_u64() as usize)
}

pub fn init(boot_info: BootInformation) -> MemorySet {
    assert_has_not_been_called!("memory::init must be called only once");

    info!("{:?}", boot_info);

    init_frame_allocator(&boot_info);

    let kernel_memory = remap_the_kernel(boot_info);

    use consts::{KERNEL_HEAP_OFFSET, KERNEL_HEAP_SIZE};

    unsafe { HEAP_ALLOCATOR.lock().init(KERNEL_HEAP_OFFSET, KERNEL_HEAP_SIZE); }

    *STACK_ALLOCATOR.lock() = Some({
        let stack_alloc_range = Page::range_of(KERNEL_HEAP_OFFSET + KERNEL_HEAP_SIZE,
                                               KERNEL_HEAP_OFFSET + KERNEL_HEAP_SIZE + 0x1000000);
        stack_allocator::StackAllocator::new(stack_alloc_range)
    });

    kernel_memory
}

pub struct BitAllocGuard(MutexGuard<'static, BitAlloc64K>);

impl FrameAllocator<Size4KiB> for BitAllocGuard {
    fn alloc(&mut self) -> Option<Frame> {
        self.0.alloc().map(|x| Frame::of_addr(x * PAGE_SIZE))
    }
}

impl FrameDeallocator<Size4KiB> for BitAllocGuard {
    fn dealloc(&mut self, frame: Frame) {
        self.0.dealloc(frame.start_address().as_u64() as usize / PAGE_SIZE);
    }
}

fn init_frame_allocator(boot_info: &BootInformation) {
    let memory_areas = boot_info.memory_map_tag().expect("Memory map tag required")
        .memory_areas();
    let elf_sections = boot_info.elf_sections_tag().expect("Elf sections tag required")
        .sections().filter(|s| s.is_allocated());

    let mut ba = FRAME_ALLOCATOR.lock();
    for area in memory_areas {
        ba.insert(to_range(area.start_address(), area.end_address()));
    }
    for section in elf_sections {
        ba.remove(to_range(section.start_address() as usize, section.end_address() as usize));
    }
    ba.remove(to_range(boot_info.start_address(), boot_info.end_address()));

    use core::ops::Range;
    fn to_range(mut start_addr: usize, mut end_addr: usize) -> Range<usize> {
        use consts::KERNEL_OFFSET;
        if start_addr >= KERNEL_OFFSET {
            start_addr -= KERNEL_OFFSET;
        }
        if end_addr >= KERNEL_OFFSET {
            end_addr -= KERNEL_OFFSET;
        }
        let page_start = start_addr / PAGE_SIZE;
        let mut page_end = (end_addr - 1) / PAGE_SIZE + 1;
        if page_end >= BitAlloc64K::CAP {
            warn!("page num {:#x} out of range {:#x}", page_end, BitAlloc64K::CAP);
            page_end = BitAlloc64K::CAP;
        }
        page_start..page_end
    }
}

fn remap_the_kernel(boot_info: BootInformation) -> MemorySet {
    let mut memory_set = MemorySet::from(boot_info.elf_sections_tag().unwrap());

    use consts::{KERNEL_OFFSET, KERNEL_HEAP_OFFSET, KERNEL_HEAP_SIZE};
    memory_set.push(MemoryArea::new_kernel(KERNEL_OFFSET + 0xb8000, KERNEL_OFFSET + 0xb9000, MemoryAttr::default(), "VGA"));
    memory_set.push(MemoryArea::new_kernel(KERNEL_OFFSET + 0xfee00000, KERNEL_OFFSET + 0xfee01000, MemoryAttr::default(), "LAPIC"));
    memory_set.push(MemoryArea::new(KERNEL_HEAP_OFFSET, KERNEL_HEAP_OFFSET + KERNEL_HEAP_SIZE, MemoryAttr::default(), "kernel_heap"));
    debug!("{:#x?}", memory_set);

    memory_set.switch();
    info!("NEW TABLE!!!");

    let kstack = get_init_kstack_and_set_guard_page();
    memory_set.set_kstack(kstack);

    memory_set
}

fn get_init_kstack_and_set_guard_page() -> Stack {
    assert_has_not_been_called!();

    extern { fn stack_bottom(); }
    let stack_bottom = PhysAddr::new(stack_bottom as u64).to_kernel_virtual();

    // turn the stack bottom into a guard page
    active_table().unmap(stack_bottom);
    debug!("guard page at {:?}", stack_bottom);

    Stack::new(stack_bottom + 8 * PAGE_SIZE, stack_bottom + 1 * PAGE_SIZE)
}

impl From<ElfSectionsTag> for MemorySet {
    fn from(sections: ElfSectionsTag) -> Self {
        assert_has_not_been_called!();
        // WARNING: must ensure it's large enough
        static mut SPACE: [u8; 0x1000] = [0; 0x1000];
        let mut set = unsafe { MemorySet::new_from_raw_space(&mut SPACE) };
        for section in sections.sections().filter(|s| s.is_allocated()) {
            set.push(MemoryArea::from(section));
        }
        set
    }
}

impl From<ElfSection> for MemoryArea {
    fn from(section: ElfSection) -> Self {
        let mut start_addr = section.start_address() as usize;
        let mut end_addr = section.end_address() as usize;
        assert_eq!(start_addr % PAGE_SIZE, 0, "sections need to be page aligned");
        let name = unsafe { &*(section.name() as *const str) };
        if start_addr < KERNEL_OFFSET {
            start_addr += KERNEL_OFFSET;
            end_addr += KERNEL_OFFSET;
        }
        MemoryArea::new_kernel(start_addr, end_addr, MemoryAttr::from(section.flags()), name)
    }
}

impl From<ElfSectionFlags> for MemoryAttr {
    fn from(elf_flags: ElfSectionFlags) -> Self {
        let mut flags = MemoryAttr::default();

        if !elf_flags.contains(ElfSectionFlags::ALLOCATED) { flags = flags.hide(); }
        if !elf_flags.contains(ElfSectionFlags::WRITABLE) { flags = flags.readonly(); }
        if elf_flags.contains(ElfSectionFlags::EXECUTABLE) { flags = flags.execute(); }
        flags
    }
}

pub mod test {
    pub fn cow() {
        use super::*;
        use ucore_memory::cow::test::test_with;
        test_with(&mut active_table());
    }
}