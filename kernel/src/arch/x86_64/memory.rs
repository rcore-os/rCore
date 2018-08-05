use bit_allocator::{BitAlloc, BitAlloc64K};
use consts::KERNEL_OFFSET;
// Depends on kernel
use memory::{active_table, FRAME_ALLOCATOR, init_heap, MemoryArea, MemoryAttr, MemorySet, Stack};
use super::multiboot2::{ElfSection, ElfSectionFlags, ElfSectionsTag};
use super::multiboot2::BootInformation;
use ucore_memory::PAGE_SIZE;
use ucore_memory::paging::PageTable;

// BootInformation may trigger page fault after kernel remap
// So just take its ownership
pub fn init(boot_info: BootInformation) {
    assert_has_not_been_called!("memory::init must be called only once");
    info!("{:?}", boot_info);
    init_frame_allocator(&boot_info);
    remap_the_kernel(&boot_info);
    init_heap();
}

fn init_frame_allocator(boot_info: &BootInformation) {
    let memory_areas = boot_info.memory_map_tag().expect("Memory map tag required")
        .memory_areas();
    let elf_sections = boot_info.elf_sections_tag().expect("Elf sections tag required")
        .sections().filter(|s| s.is_allocated());

    let mut ba = FRAME_ALLOCATOR.lock();
    for area in memory_areas {
        ba.insert(to_range(area.start_address() as usize, area.end_address() as usize));
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

fn remap_the_kernel(boot_info: &BootInformation) {
    extern { fn stack_bottom(); }
    extern { fn stack_top(); }
    let kstack = Stack {
        top: stack_top as usize + KERNEL_OFFSET,
        bottom: stack_bottom as usize + PAGE_SIZE + KERNEL_OFFSET,
    };

    let mut memory_set = memory_set_from(boot_info.elf_sections_tag().unwrap(), kstack);

    use consts::{KERNEL_HEAP_OFFSET, KERNEL_HEAP_SIZE};
    use super::smp::ENTRYOTHER_ADDR;
    memory_set.push(MemoryArea::new_physical(0xb8000, 0xb9000, KERNEL_OFFSET, MemoryAttr::default(), "VGA"));
    memory_set.push(MemoryArea::new_physical(0xfee00000, 0xfee01000, KERNEL_OFFSET, MemoryAttr::default(), "LAPIC"));
    memory_set.push(MemoryArea::new_identity(0x07fe1000, 0x07fe1000 + PAGE_SIZE, MemoryAttr::default(), "RSDT"));
    memory_set.push(MemoryArea::new_identity(0xfec00000, 0xfec00000 + PAGE_SIZE, MemoryAttr::default(), "IOAPIC"));
    memory_set.push(MemoryArea::new(KERNEL_HEAP_OFFSET, KERNEL_HEAP_OFFSET + KERNEL_HEAP_SIZE, MemoryAttr::default(), "kernel_heap"));
    memory_set.push(MemoryArea::new_identity(ENTRYOTHER_ADDR, ENTRYOTHER_ADDR + PAGE_SIZE, MemoryAttr::default().execute(), "entry_other.text"));
    memory_set.push(MemoryArea::new_physical(0, 4096, KERNEL_OFFSET, MemoryAttr::default(), "entry_other.ctrl"));
    debug!("{:#x?}", memory_set);

    unsafe { memory_set.activate(); }
    info!("NEW TABLE!!!");

    use core::mem::forget;
    forget(memory_set);
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