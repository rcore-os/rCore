pub use self::area_frame_allocator::AreaFrameAllocator;
pub use arch::paging::*;
pub use self::stack_allocator::Stack;
pub use self::address::*;
pub use self::frame::*;
pub use self::memory_set::*;

use multiboot2::BootInformation;
use consts::KERNEL_OFFSET;
use arch::paging;
use arch::paging::EntryFlags;
use spin::Mutex;

mod memory_set;
mod area_frame_allocator;
pub mod heap_allocator;
mod stack_allocator;
mod address;
mod frame;

pub static FRAME_ALLOCATOR: Mutex<Option<AreaFrameAllocator>> = Mutex::new(None);

pub fn alloc_frame() -> Frame {
    FRAME_ALLOCATOR.lock()
        .as_mut().expect("frame allocator is not initialized")
        .allocate_frame().expect("no more frame")
}

pub fn init(boot_info: &BootInformation) -> MemoryController {
    assert_has_not_been_called!("memory::init must be called only once");

    let memory_map_tag = boot_info.memory_map_tag().expect(
        "Memory map tag required");
    let elf_sections_tag = boot_info.elf_sections_tag().expect(
        "Elf sections tag required");

    let kernel_start = PhysAddr(elf_sections_tag.sections()
        .filter(|s| s.is_allocated()).map(|s| s.start_address()).min().unwrap() as u64);
    let kernel_end = PhysAddr::from_kernel_virtual(elf_sections_tag.sections()
        .filter(|s| s.is_allocated()).map(|s| s.end_address()).max().unwrap());

    let boot_info_start = PhysAddr(boot_info.start_address() as u64);
    let boot_info_end = PhysAddr(boot_info.end_address() as u64);

    *FRAME_ALLOCATOR.lock() = Some(AreaFrameAllocator::new(
        kernel_start, kernel_end,
        boot_info_start, boot_info_end,
        memory_map_tag.memory_areas()
    ));

    let (mut active_table, kernel_stack) = remap_the_kernel(boot_info);

    use self::paging::Page;
    use consts::{KERNEL_HEAP_OFFSET, KERNEL_HEAP_SIZE};

    let stack_allocator = {
        let stack_alloc_range = Page::range_of(KERNEL_HEAP_OFFSET + KERNEL_HEAP_SIZE,
                                               KERNEL_HEAP_OFFSET + KERNEL_HEAP_SIZE + 0x100000);
        stack_allocator::StackAllocator::new(stack_alloc_range)
    };
    
    MemoryController {
        kernel_stack: Some(kernel_stack),
        active_table,
        stack_allocator,
    }
}

pub fn remap_the_kernel(boot_info: &BootInformation) -> (ActivePageTable, Stack)
{
    let mut active_table = unsafe { ActivePageTable::new() };
    let mut memory_set = MemorySet::from(boot_info.elf_sections_tag().unwrap());

    use consts::{KERNEL_HEAP_OFFSET, KERNEL_HEAP_SIZE};
    memory_set.push(MemoryArea {
        start_addr: 0xb8000,
        end_addr: 0xb9000,
        phys_start_addr: Some(PhysAddr(0xb8000)),
        flags: EntryFlags::WRITABLE.bits() as u32,
        name: "VGA",
        mapped: false,
    });
    memory_set.push(MemoryArea {
        start_addr: boot_info.start_address(),
        end_addr: boot_info.end_address(),
        phys_start_addr: Some(PhysAddr(boot_info.start_address() as u64)),
        flags: EntryFlags::PRESENT.bits() as u32,
        name: "multiboot",
        mapped: false,
    });
    memory_set.push(MemoryArea {
        start_addr: KERNEL_HEAP_OFFSET,
        end_addr: KERNEL_HEAP_OFFSET + KERNEL_HEAP_SIZE,
        phys_start_addr: None,
        flags: EntryFlags::WRITABLE.bits() as u32,
        name: "kernel_heap",
        mapped: false,
    });
    memory_set.map(&mut active_table);
    println!("{:#x?}", memory_set);

    let old_table = active_table.switch(memory_set.page_table.take().unwrap());
    println!("NEW TABLE!!!");

    // turn the stack bottom into a guard page
    extern { fn stack_bottom(); }
    let stack_bottom = PhysAddr(stack_bottom as u64).to_kernel_virtual();
    let stack_bottom_page = Page::of_addr(stack_bottom);
    active_table.unmap(stack_bottom_page);
    let kernel_stack = Stack::new(stack_bottom + 8 * PAGE_SIZE, stack_bottom + 1 * PAGE_SIZE);
    println!("guard page at {:#x}", stack_bottom_page.start_address());

    (active_table, kernel_stack)
}

use multiboot2::{ElfSectionsTag, ElfSection, ElfSectionFlags};

impl From<&'static ElfSectionsTag> for MemorySet {
    fn from(sections: &'static ElfSectionsTag) -> Self {
        assert_has_not_been_called!();
        // WARNING: must ensure it's large enough
        static mut SPACE: [u8; 0x1000] = [0; 0x1000];
        let mut set = unsafe { MemorySet::new_from_raw_space(&mut SPACE) };
        for section in sections.sections() {
            if !section.is_allocated() {
                // section is not loaded to memory
                continue;
            }
            set.push(MemoryArea::from(section));
        }
        set
    }
}

impl<'a> From<&'a ElfSection> for MemoryArea {
    fn from(section: &'a ElfSection) -> Self {
        use self::address::FromToVirtualAddress;
        assert_eq!(section.start_address() % PAGE_SIZE, 0, "sections need to be page aligned");
        if section.start_address() < KERNEL_OFFSET {
            MemoryArea {
                start_addr: section.start_address() + KERNEL_OFFSET,
                end_addr: section.end_address() + KERNEL_OFFSET,
                phys_start_addr: Some(PhysAddr(section.start_address() as u64)),
                flags: EntryFlags::from(section.flags()).bits() as u32,
                name: "",
                mapped: false,
            }
        } else {
            MemoryArea {
                start_addr: section.start_address(),
                end_addr: section.end_address(),
                phys_start_addr: Some(PhysAddr::from_kernel_virtual(section.start_address())),
                flags: EntryFlags::from(section.flags()).bits() as u32,
                name: "",
                mapped: false,
            }
        }
    }
}

impl From<ElfSectionFlags> for EntryFlags {
    fn from(elf_flags: ElfSectionFlags) -> Self {
        use multiboot2::{ELF_SECTION_ALLOCATED, ELF_SECTION_WRITABLE,
                         ELF_SECTION_EXECUTABLE};

        let mut flags = EntryFlags::empty();

        if elf_flags.contains(ELF_SECTION_ALLOCATED) {
            // section is loaded to memory
            flags = flags | EntryFlags::PRESENT;
        }
        if elf_flags.contains(ELF_SECTION_WRITABLE) {
            flags = flags | EntryFlags::WRITABLE;
        }
        if !elf_flags.contains(ELF_SECTION_EXECUTABLE) {
            flags = flags | EntryFlags::NO_EXECUTE;
        }
        flags
    }
}

pub struct MemoryController {
    pub kernel_stack: Option<Stack>,
    active_table: paging::ActivePageTable,
    stack_allocator: stack_allocator::StackAllocator,
}

impl MemoryController {
    pub fn alloc_stack(&mut self, size_in_pages: usize) -> Option<Stack> {
        let &mut MemoryController { ref mut kernel_stack,
                                    ref mut active_table,
                                    ref mut stack_allocator } = self;
        stack_allocator.alloc_stack(active_table, size_in_pages)
    }
    pub fn new_page_table(&mut self) -> InactivePageTable {
        let frame = alloc_frame();
        let page_table = InactivePageTable::new(frame, &mut self.active_table);
        page_table
    }
    pub fn map_page_identity(&mut self, addr: usize) {
        let frame = Frame::of_addr(addr);
        let flags = EntryFlags::WRITABLE;
        self.active_table.identity_map(frame, flags);
    }
    pub fn map_page_p2v(&mut self, addr: PhysAddr) {
        let page = Page::of_addr(addr.to_kernel_virtual());
        let frame = Frame::of_addr(addr.get());
        let flags = EntryFlags::WRITABLE;
        self.active_table.map_to(page, frame, flags);
    }
}
