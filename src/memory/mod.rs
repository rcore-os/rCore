pub use arch::paging::*;
use bit_allocator::{BitAlloc, BitAlloc64K};
use consts::KERNEL_OFFSET;
use multiboot2::{ElfSection, ElfSectionFlags, ElfSectionsTag};
use multiboot2::BootInformation;
pub use self::stack_allocator::*;
use spin::{Mutex, MutexGuard};
use super::HEAP_ALLOCATOR;
use ucore_memory::{*, paging::PageTable, cow::CowExt};
pub use ucore_memory::memory_set::{MemoryAttr, MemoryArea, MemorySet as MemorySet_, Stack};

pub type MemorySet = MemorySet_<InactivePageTable0>;

mod stack_allocator;

lazy_static! {
    static ref FRAME_ALLOCATOR: Mutex<BitAlloc64K> = Mutex::new(BitAlloc64K::default());
}
static STACK_ALLOCATOR: Mutex<Option<StackAllocator>> = Mutex::new(None);

pub fn alloc_frame() -> Option<usize> {
    FRAME_ALLOCATOR.lock().alloc().map(|id| id * PAGE_SIZE)
}

pub fn dealloc_frame(target: usize) {
    FRAME_ALLOCATOR.lock().dealloc(target / PAGE_SIZE);
}

pub fn alloc_stack(size_in_pages: usize) -> Stack {
    STACK_ALLOCATOR.lock()
        .as_mut().expect("stack allocator is not initialized")
        .alloc_stack(size_in_pages).expect("no more stack")
}

lazy_static! {
    static ref ACTIVE_TABLE: Mutex<CowExt<ActivePageTable>> = Mutex::new(unsafe {
        CowExt::new(ActivePageTable::new())
    });
}

/// The only way to get active page table
pub fn active_table() -> MutexGuard<'static, CowExt<ActivePageTable>> {
    ACTIVE_TABLE.lock()
}

// Return true to continue, false to halt
pub fn page_fault_handler(addr: usize) -> bool {
    // Handle copy on write
    unsafe { ACTIVE_TABLE.force_unlock(); }
    active_table().page_fault_handler(addr, || alloc_frame().unwrap())
}

pub fn init(boot_info: BootInformation) -> MemorySet {
    assert_has_not_been_called!("memory::init must be called only once");

    info!("{:?}", boot_info);

    init_frame_allocator(&boot_info);

    let kernel_memory = remap_the_kernel(boot_info);

    use consts::{KERNEL_HEAP_OFFSET, KERNEL_HEAP_SIZE};

    unsafe { HEAP_ALLOCATOR.lock().init(KERNEL_HEAP_OFFSET, KERNEL_HEAP_SIZE); }

    *STACK_ALLOCATOR.lock() = Some({
        use ucore_memory::Page;
        let stack_alloc_range = Page::range_of(KERNEL_HEAP_OFFSET + KERNEL_HEAP_SIZE,
                                               KERNEL_HEAP_OFFSET + KERNEL_HEAP_SIZE + 0x1000000);
        stack_allocator::StackAllocator::new(stack_alloc_range)
    });

    kernel_memory
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
    extern { fn stack_bottom(); }
    let stack_bottom = stack_bottom as usize + KERNEL_OFFSET;
    let kstack = Stack {
        top: stack_bottom + 8 * PAGE_SIZE,
        bottom: stack_bottom + 1 * PAGE_SIZE,
    };

    let mut memory_set = memory_set_from(boot_info.elf_sections_tag().unwrap(), kstack);

    use consts::{KERNEL_OFFSET, KERNEL_HEAP_OFFSET, KERNEL_HEAP_SIZE};
    memory_set.push(MemoryArea::new_physical(0xb8000, 0xb9000, KERNEL_OFFSET, MemoryAttr::default(), "VGA"));
    memory_set.push(MemoryArea::new_physical(0xfee00000, 0xfee01000, KERNEL_OFFSET, MemoryAttr::default(), "LAPIC"));
    memory_set.push(MemoryArea::new(KERNEL_HEAP_OFFSET, KERNEL_HEAP_OFFSET + KERNEL_HEAP_SIZE, MemoryAttr::default(), "kernel_heap"));
    debug!("{:#x?}", memory_set);

    unsafe { memory_set.activate(); }
    info!("NEW TABLE!!!");

    // turn the stack bottom into a guard page
    active_table().unmap(stack_bottom);
    debug!("guard page at {:?}", stack_bottom);

    memory_set
}

fn memory_set_from(sections: ElfSectionsTag, kstack: Stack) -> MemorySet {
    assert_has_not_been_called!();
    // WARNING: must ensure it's large enough
    static mut SPACE: [u8; 0x1000] = [0; 0x1000];
    let mut set = unsafe { MemorySet::new_from_raw_space(&mut SPACE, kstack) };
    for section in sections.sections().filter(|s| s.is_allocated()) {
        set.push(memory_area_from(section));
    }
    set
}

fn memory_area_from(section: ElfSection) -> MemoryArea {
    let mut start_addr = section.start_address() as usize;
    let mut end_addr = section.end_address() as usize;
    assert_eq!(start_addr % PAGE_SIZE, 0, "sections need to be page aligned");
    let name = unsafe { &*(section.name() as *const str) };
    if start_addr >= KERNEL_OFFSET {
        start_addr -= KERNEL_OFFSET;
        end_addr -= KERNEL_OFFSET;
    }
    MemoryArea::new_physical(start_addr, end_addr, KERNEL_OFFSET, memory_attr_from(section.flags()), name)
}

fn memory_attr_from(elf_flags: ElfSectionFlags) -> MemoryAttr {
    let mut flags = MemoryAttr::default();

    if !elf_flags.contains(ElfSectionFlags::ALLOCATED) { flags = flags.hide(); }
    if !elf_flags.contains(ElfSectionFlags::WRITABLE) { flags = flags.readonly(); }
    if elf_flags.contains(ElfSectionFlags::EXECUTABLE) { flags = flags.execute(); }
    flags
}

pub mod test {
    pub fn cow() {
        use super::*;
        use ucore_memory::cow::test::test_with;
        test_with(&mut active_table());
    }
}