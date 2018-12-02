use crate::arch::interrupt::{TrapFrame, Context as ArchContext};
use crate::memory::{MemoryArea, MemoryAttr, MemorySet, KernelStack, active_table_swap, alloc_frame, InactivePageTable0, memory_set_record};
use xmas_elf::{ElfFile, header, program::{Flags, ProgramHeader, Type}};
use core::fmt::{Debug, Error, Formatter};
use alloc::{boxed::Box, collections::BTreeMap, vec::Vec, sync::Arc, string::String};
use ucore_memory::{Page};
use ucore_memory::memory_set::*;
use ucore_process::Context;
use simple_filesystem::file::File;
use spin::Mutex;
use log::*;


// TODO: avoid pub
pub struct ContextImpl {
    pub arch: ArchContext,
    pub memory_set: MemorySet,
    pub kstack: KernelStack,
    pub files: BTreeMap<usize, Arc<Mutex<File>>>,
    pub cwd: String,
}

impl Context for ContextImpl {
    unsafe fn switch_to(&mut self, target: &mut Context) {
        use core::mem::transmute;
        let (target, _): (&mut ContextImpl, *const ()) = transmute(target);
        self.arch.switch(&mut target.arch);
    }
}

impl ContextImpl {
    pub unsafe fn new_init() -> Box<Context> {
        Box::new(ContextImpl {
            arch: ArchContext::null(),
            memory_set: MemorySet::new(),
            kstack: KernelStack::new(),
            files: BTreeMap::default(),
            cwd: String::new(),
        })
    }

    pub fn new_kernel(entry: extern fn(usize) -> !, arg: usize) -> Box<Context> {
        let memory_set = MemorySet::new();
        let kstack = KernelStack::new();
        Box::new(ContextImpl {
            arch: unsafe { ArchContext::new_kernel_thread(entry, arg, kstack.top(), memory_set.token()) },
            memory_set,
            kstack,
            files: BTreeMap::default(),
            cwd: String::new(),
        })
    }

    /// Temp for aarch64
    pub fn new_user_test(entry: extern fn(usize) -> !) -> Box<Context> {
        let memory_set = MemorySet::new();
        let kstack = KernelStack::new();
        let ustack = KernelStack::new();
        Box::new(ContextImpl {
            arch: unsafe { ArchContext::new_user_thread(entry as usize, ustack.top(), kstack.top(), false, memory_set.token()) },
            memory_set,
            kstack,
            files: BTreeMap::default(),
            cwd: String::new(),
        })
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
    pub fn new_user<'a, Iter>(data: &[u8], args: Iter) -> Box<ContextImpl>
        where Iter: Iterator<Item=&'a str>
    {
        // Parse elf
        let elf = ElfFile::new(data).expect("failed to read elf");
        let is32 = match elf.header.pt2 {
            header::HeaderPt2::Header32(_) => true,
            header::HeaderPt2::Header64(_) => false,
        };
        assert_eq!(elf.header.pt2.type_().as_type(), header::Type::Executable, "ELF is not executable");

        // User stack
        use crate::consts::{USER_STACK_OFFSET, USER_STACK_SIZE, USER32_STACK_OFFSET};
        let (ustack_buttom, mut ustack_top) = match is32 {
            true => (USER32_STACK_OFFSET, USER32_STACK_OFFSET + USER_STACK_SIZE),
            false => (USER_STACK_OFFSET, USER_STACK_OFFSET + USER_STACK_SIZE),
        };

        // Make page table
        let mut memory_set = memory_set_from(&elf);

        // add the new memory set to the recorder
        let mmset_ptr = ((&mut memory_set) as * mut MemorySet) as usize;
        memory_set_record().push_back(mmset_ptr);
        //let id = memory_set_record().iter()
        //    .position(|x| unsafe { info!("current memory set record include {:x?}, {:x?}", x, (*(x.clone() as *mut MemorySet)).get_page_table_mut().token()); false });

        memory_set.push(MemoryArea::new(ustack_buttom, ustack_top, MemoryAttr::default().user(), "user_stack"));
        trace!("{:#x?}", memory_set);

        let entry_addr = elf.header.pt2.entry_point() as usize;

        // Temporary switch to it, in order to copy data
        unsafe {
            memory_set.with(|| {
                for ph in elf.program_iter() {
                    if ph.get_type() != Ok(Type::Load) {
                        continue;
                    }

                    let virt_addr = ph.virtual_addr() as usize;
                    let offset = ph.offset() as usize;
                    let file_size = ph.file_size() as usize;
                    let mem_size = ph.mem_size() as usize;

                    let target = unsafe { ::core::slice::from_raw_parts_mut(virt_addr as *mut u8, mem_size) };
                    if file_size != 0 {
                        target[..file_size].copy_from_slice(&data[offset..offset + file_size]);
                    }
                    target[file_size..].iter_mut().for_each(|x| *x = 0);
                }
                ustack_top = push_args_at_stack(args, ustack_top);
            });
        }

        let kstack = KernelStack::new();
        {
            let mut mmset_record = memory_set_record();
            let id = mmset_record.iter()
                .position(|x| x.clone() == mmset_ptr).expect("id not exist");
            mmset_record.remove(id);
        }

        let mut ret = Box::new(ContextImpl {
            arch: unsafe {
                ArchContext::new_user_thread(
                    entry_addr, ustack_top, kstack.top(), is32, memory_set.token())
            },
            memory_set,
            kstack,
            files: BTreeMap::default(),
            cwd: String::new(),
        });
        //set the user Memory pages in the memory set swappable
        memory_set_map_swappable(ret.get_memory_set_mut());
        ret
    }

    /// Fork
    pub fn fork(&self, tf: &TrapFrame) -> Box<Context> {
        info!("COME into fork!");
        // Clone memory set, make a new page table
        let mut memory_set = self.memory_set.clone();
        info!("finish mmset clone in fork!");
        // add the new memory set to the recorder
        info!("fork! new page table token: {:x?}", memory_set.token());
        let mmset_ptr = ((&mut memory_set) as * mut MemorySet) as usize;
        memory_set_record().push_back(mmset_ptr);

        info!("before copy data to temp space");
        // Copy data to temp space
        use alloc::vec::Vec;
        let datas: Vec<Vec<u8>> = memory_set.iter().map(|area| {
            Vec::from(unsafe { area.as_slice() })
        }).collect();

        info!("Finish copy data to temp space.");

        // Temporarily switch to it, in order to copy data
        unsafe {
            memory_set.with(|| {
                for (area, data) in memory_set.iter().zip(datas.iter()) {
                    area.as_slice_mut().copy_from_slice(data.as_slice())
                }
            });
        }

        info!("temporary copy data!");
        let kstack = KernelStack::new();

        // remove the raw pointer for the memory set in memory_set_record
        {
            let mut mmset_record = memory_set_record();
            let id = mmset_record.iter()
                .position(|x| x.clone() == mmset_ptr).expect("id not exist");
            mmset_record.remove(id);
        }


        let mut ret = Box::new(ContextImpl {
            arch: unsafe { ArchContext::new_fork(tf, kstack.top(), memory_set.token()) },
            memory_set,
            kstack,
            files: BTreeMap::default(),
            cwd: String::new(),
        });

        memory_set_map_swappable(ret.get_memory_set_mut());
        info!("FORK() finsihed!");
        ret
    }

    pub fn get_memory_set_mut(&mut self) -> &mut MemorySet {
        &mut self.memory_set
    }

}

impl Drop for ContextImpl{
    fn drop(&mut self){
        info!("come in to drop for ContextImpl");
        //set the user Memory pages in the memory set unswappable
        let Self {ref mut arch, ref mut memory_set, ref mut kstack, ..} = self;
        let pt = {
            memory_set.get_page_table_mut() as *mut InactivePageTable0
        };
        for area in memory_set.iter(){
            for page in Page::range_of(area.get_start_addr(), area.get_end_addr()) {
                let addr = page.start_address();
                unsafe {
                    active_table_swap().remove_from_swappable(pt, addr, || alloc_frame().expect("alloc frame failed"));
                }
            }
        }
        debug!("Finishing setting pages unswappable");
    }
}

impl Debug for ContextImpl {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{:x?}", self.arch)
    }
}

/// Push a slice at the stack. Return the new sp.
unsafe fn push_slice<T: Copy>(mut sp: usize, vs: &[T]) -> usize {
    use core::{mem::{size_of, align_of}, slice};
    sp -= vs.len() * size_of::<T>();
    sp -= sp % align_of::<T>();
    slice::from_raw_parts_mut(sp as *mut T, vs.len())
        .copy_from_slice(vs);
    sp
}

unsafe fn push_args_at_stack<'a, Iter>(args: Iter, stack_top: usize) -> usize
    where Iter: Iterator<Item=&'a str>
{
    use core::{ptr, slice};
    let mut sp = stack_top;
    let mut argv = Vec::new();
    for arg in args {
        sp = push_slice(sp, &[0u8]);
        sp = push_slice(sp, arg.as_bytes());
        argv.push(sp);
    }
    sp = push_slice(sp, argv.as_slice());
    sp = push_slice(sp, &[argv.len()]);
    sp
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
    debug!("come in to memory_set_from");
    let mut set = MemorySet::new();
    for ph in elf.program_iter() {
        if ph.get_type() != Ok(Type::Load) {
            continue;
        }
        let (virt_addr, mem_size, flags) = match ph {
            ProgramHeader::Ph32(ph) => (ph.virtual_addr as usize, ph.mem_size as usize, ph.flags),
            ProgramHeader::Ph64(ph) => (ph.virtual_addr as usize, ph.mem_size as usize, ph.flags),
        };
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
pub fn memory_set_map_swappable(memory_set: &mut MemorySet){
    info!("COME INTO memory set map swappable!");
    let pt = unsafe {
        memory_set.get_page_table_mut() as *mut InactivePageTable0
    };
    for area in memory_set.iter(){
        for page in Page::range_of(area.get_start_addr(), area.get_end_addr()) {
            let addr = page.start_address();
            unsafe { active_table_swap().set_swappable(pt, addr); }
        }
    }
    info!("Finishing setting pages swappable");
}

