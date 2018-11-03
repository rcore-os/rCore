use arch::interrupt::{TrapFrame, Context as ArchContext};
use memory::{MemoryArea, MemoryAttr, MemorySet, active_table_swap, alloc_frame};
use xmas_elf::{ElfFile, header, program::{Flags, ProgramHeader, Type}};
use core::fmt::{Debug, Error, Formatter};
use ucore_memory::{Page};
use ::memory::{InactivePageTable0};

pub struct Context {
    arch: ArchContext,
    memory_set: MemorySet,
}

impl ::ucore_process::processor::Context for Context {
    /*
    * @param: 
    *   target: the target process context
    * @brief: 
    *   switch to the target process context
    */
    unsafe fn switch(&mut self, target: &mut Self) {
        super::PROCESSOR.try().unwrap().force_unlock();
        self.arch.switch(&mut target.arch);
        use core::mem::forget;
        // don't run the distructor of processor()
        forget(super::processor());
    }

    /*
    * @param: 
    *   entry: the program entry for the process
    *   arg: a0 (a parameter)
    * @brief: 
    *   new a kernel thread Context
    * @retval: 
    *   the new kernel thread Context
    */
    fn new_kernel(entry: extern fn(usize) -> !, arg: usize) -> Self {
        let ms = MemorySet::new();
        Context {
            arch: unsafe { ArchContext::new_kernel_thread(entry, arg, ms.kstack_top(), ms.token()) },
            memory_set: ms,
        }
    }
}

impl Context {
    pub unsafe fn new_init() -> Self {
        Context {
            arch: ArchContext::null(),
            memory_set: MemorySet::new(),
        }
    }

    /// Make a new user thread from ELF data
    /*
    * @param: 
    *   data: the ELF data stream 
    * @brief: 
    *   make a new thread from ELF data
    * @retval: 
    *   the new user thread Context
    */
    pub fn new_user(data: &[u8]) -> Self {
        info!("come into new user");
        // Parse elf
        let elf = ElfFile::new(data).expect("failed to read elf");
        let is32 = match elf.header.pt2 {
            header::HeaderPt2::Header32(_) => true,
            header::HeaderPt2::Header64(_) => false,
        };
        assert_eq!(elf.header.pt2.type_().as_type(), header::Type::Executable, "ELF is not executable");

        // User stack
        info!("start building suer stack");
        use consts::{USER_STACK_OFFSET, USER_STACK_SIZE, USER32_STACK_OFFSET};
        let (user_stack_buttom, user_stack_top) = match is32 {
            true => (USER32_STACK_OFFSET, USER32_STACK_OFFSET + USER_STACK_SIZE),
            false => (USER_STACK_OFFSET, USER_STACK_OFFSET + USER_STACK_SIZE),
        };

        // Make page table
        info!("make page table!");
        let mut memory_set = memory_set_from(&elf);
        info!("start to push user stack to the mmset");
        memory_set.push(MemoryArea::new(user_stack_buttom, user_stack_top, MemoryAttr::default().user(), "user_stack"));
        trace!("{:#x?}", memory_set);

        let entry_addr = elf.header.pt2.entry_point() as usize;

        // Temporary switch to it, in order to copy data
        info!("starting copy data.");
        unsafe {
            memory_set.with(|| {
                for ph in elf.program_iter() {
                    let virt_addr = ph.virtual_addr() as usize;
                    let offset = ph.offset() as usize;
                    let file_size = ph.file_size() as usize;
                    if file_size == 0 {
                        return;
                    }
                    info!("file virtaddr: {:x?}, file size: {:x?}", virt_addr, file_size);
                    use core::slice;
                    info!("starting copy!");
                    let target = unsafe { slice::from_raw_parts_mut(virt_addr as *mut u8, file_size) };
                    info!("target got!");
                    target.copy_from_slice(&data[offset..offset + file_size]);
                    info!("finish copy!");
                }
                if is32 {
                    unsafe {
                        // TODO: full argc & argv
                        *(user_stack_top as *mut u32).offset(-1) = 0; // argv
                        *(user_stack_top as *mut u32).offset(-2) = 0; // argc
                    }
                }
            });
        }
        info!("ending copy data.");
        
        //set the user Memory pages in the memory set swappable
        //memory_set_map_swappable(&mut memory_set);

        Context {
            arch: unsafe {
                ArchContext::new_user_thread(
                    entry_addr, user_stack_top - 8, memory_set.kstack_top(), is32, memory_set.token())
            },
            memory_set,
        }
    }

    /// Fork
    pub fn fork(&self, tf: &TrapFrame) -> Self {
        // Clone memory set, make a new page table
        let mut memory_set = self.memory_set.clone();

        // Copy data to temp space
        use alloc::vec::Vec;
        let datas: Vec<Vec<u8>> = memory_set.iter().map(|area| {
            Vec::from(unsafe { area.as_slice() })
        }).collect();

        // Temporary switch to it, in order to copy data
        unsafe {
            memory_set.with(|| {
                for (area, data) in memory_set.iter().zip(datas.iter()) {
                    unsafe { area.as_slice_mut() }.copy_from_slice(data.as_slice())
                }
            });
        }
        // map the memory set swappable
        memory_set_map_swappable(&mut memory_set);

        Context {
            arch: unsafe { ArchContext::new_fork(tf, memory_set.kstack_top(), memory_set.token()) },
            memory_set,
        }
    }

    pub fn get_memory_set_mut(&mut self) -> &mut MemorySet {
        &mut self.memory_set
    }

}

impl Drop for Context{
    fn drop(&mut self){
        /*
        //set the user Memory pages in the memory set unswappable
        let Self {ref mut arch, ref mut memory_set} = self;
        let pt = {
            memory_set.get_page_table_mut() as *mut InactivePageTable0
        };
        for area in memory_set.iter(){
            for page in Page::range_of(area.get_start_addr(), area.get_end_addr()) {
                let addr = page.start_address();
                unsafe {
                    active_table_swap().remove_from_swappable(pt, addr, || alloc_frame().unwrap());
                }
            }
        }
        info!("Finishing setting pages unswappable");
        */
    }
}

impl Debug for Context {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{:x?}", self.arch)
    }
}

/*
* @param: 
*   elf: the source ELF file
* @brief: 
*   generate a memory set according to the elf file
* @retval: 
*   the new memory set
*/
fn memory_set_from<'a>(elf: &'a ElfFile<'a>) -> MemorySet {
    info!("come in to memory_set_from");
    let mut set = MemorySet::new();
    for ph in elf.program_iter() {
        if ph.get_type() != Ok(Type::Load) {
            continue;
        }
        let (virt_addr, mem_size, flags) = match ph {
            ProgramHeader::Ph32(ph) => (ph.virtual_addr as usize, ph.mem_size as usize, ph.flags),
            ProgramHeader::Ph64(ph) => (ph.virtual_addr as usize, ph.mem_size as usize, ph.flags),//???
        };
        info!("push!");
        set.push(MemoryArea::new(virt_addr, virt_addr + mem_size, memory_attr_from(flags), ""));

    }
    set
}

fn memory_attr_from(elf_flags: Flags) -> MemoryAttr {
    let mut flags = MemoryAttr::default().user();
    // TODO: handle readonly
    if elf_flags.is_execute() { flags = flags.execute(); }
    flags
}

/*
* @param: 
*   memory_set: the target MemorySet to set swappable
* @brief:
*   map the memory area in the memory_set swappalbe, specially for the user process
*/
fn memory_set_map_swappable(memory_set: &mut MemorySet){
    let pt = unsafe {
        memory_set.get_page_table_mut() as *mut InactivePageTable0
    };
    for area in memory_set.iter(){
        for page in Page::range_of(area.get_start_addr(), area.get_end_addr()) {
            let addr = page.start_address();
            active_table_swap().set_swappable(pt, addr);
        }
    }
    info!("Finishing setting pages swappable");
}