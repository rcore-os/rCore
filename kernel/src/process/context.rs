use arch::interrupt::{TrapFrame, Context as ArchContext};
use memory::{MemoryArea, MemoryAttr, MemorySet};
use xmas_elf::{ElfFile, header, program::{Flags, ProgramHeader, Type}};
use core::fmt::{Debug, Error, Formatter};
use ucore_process::Context;
use alloc::boxed::Box;

pub struct ContextImpl {
    arch: ArchContext,
    memory_set: MemorySet,
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
        })
    }

    pub fn new_kernel(entry: extern fn(usize) -> !, arg: usize) -> Box<Context> {
        let ms = MemorySet::new();
        Box::new(ContextImpl {
            arch: unsafe { ArchContext::new_kernel_thread(entry, arg, ms.kstack_top(), ms.token()) },
            memory_set: ms,
        })
    }

    /// Make a new user thread from ELF data
    pub fn new_user(data: &[u8]) -> Box<Context> {
        // Parse elf
        let elf = ElfFile::new(data).expect("failed to read elf");
        let is32 = match elf.header.pt2 {
            header::HeaderPt2::Header32(_) => true,
            header::HeaderPt2::Header64(_) => false,
        };
        assert_eq!(elf.header.pt2.type_().as_type(), header::Type::Executable, "ELF is not executable");

        // User stack
        use consts::{USER_STACK_OFFSET, USER_STACK_SIZE, USER32_STACK_OFFSET};
        let (user_stack_buttom, user_stack_top) = match is32 {
            true => (USER32_STACK_OFFSET, USER32_STACK_OFFSET + USER_STACK_SIZE),
            false => (USER_STACK_OFFSET, USER_STACK_OFFSET + USER_STACK_SIZE),
        };

        // Make page table
        let mut memory_set = memory_set_from(&elf);
        memory_set.push(MemoryArea::new(user_stack_buttom, user_stack_top, MemoryAttr::default().user(), "user_stack"));
        trace!("{:#x?}", memory_set);

        let entry_addr = elf.header.pt2.entry_point() as usize;

        // Temporary switch to it, in order to copy data
        unsafe {
            memory_set.with(|| {
                for ph in elf.program_iter() {
                    let virt_addr = ph.virtual_addr() as usize;
                    let offset = ph.offset() as usize;
                    let file_size = ph.file_size() as usize;
                    if file_size == 0 {
                        return;
                    }
                    use core::slice;
                    let target = unsafe { slice::from_raw_parts_mut(virt_addr as *mut u8, file_size) };
                    target.copy_from_slice(&data[offset..offset + file_size]);
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

        Box::new(ContextImpl {
            arch: unsafe {
                ArchContext::new_user_thread(
                    entry_addr, user_stack_top - 8, memory_set.kstack_top(), is32, memory_set.token())
            },
            memory_set,
        })
    }

    /// Fork
    pub fn fork(&self, tf: &TrapFrame) -> Box<Context> {
        // Clone memory set, make a new page table
        let memory_set = self.memory_set.clone();

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

        Box::new(ContextImpl {
            arch: unsafe { ArchContext::new_fork(tf, memory_set.kstack_top(), memory_set.token()) },
            memory_set,
        })
    }
}

impl Debug for ContextImpl {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{:x?}", self.arch)
    }
}

fn memory_set_from<'a>(elf: &'a ElfFile<'a>) -> MemorySet {
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