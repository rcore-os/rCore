use super::*;
use memory::Stack;
use xmas_elf::{ElfFile, program::{Flags, ProgramHeader}};
use core::slice;
use alloc::rc::Rc;

#[derive(Debug)]
pub struct Process {
    pub(in process) pid: Pid,
                    name: &'static str,
                    kstack: Stack,
    //                    memory_set: Rc<MemorySet>,
    pub(in process) status: Status,
    pub(in process) rsp: usize,
    pub(in process) is_user: bool,
}

pub type Pid = usize;

#[derive(Debug)]
pub enum Status {
    Ready, Running, Sleeping(usize), Exited
}

impl Process {
    /// Make a new kernel thread
    pub fn new(name: &'static str, entry: extern fn(), mc: &mut MemoryController) -> Self {
        let kstack = mc.alloc_stack(7).unwrap();
        let tf = TrapFrame::new_kernel_thread(entry, kstack.top());
        let rsp = kstack.push_at_top(tf);

        Process {
            pid: 0,
            name,
            kstack,
            status: Status::Ready,
            rsp,
            is_user: false,
        }
    }
    /// Make the first kernel thread `initproc`
    /// Should be called only once
    pub fn new_init(mc: &mut MemoryController) -> Self {
        assert_has_not_been_called!();
        Process {
            pid: 0,
            name: "init",
            kstack: mc.kernel_stack.take().unwrap(),
            status: Status::Running,
            rsp: 0, // will be set at first schedule
            is_user: false,
        }
    }

    pub fn new_user(begin: usize, end: usize, mc: &mut MemoryController) -> Self {
        let slice = unsafe{ slice::from_raw_parts(begin as *const u8, end - begin) };
        let elf = ElfFile::new(slice).expect("failed to read elf");
        let phys_start = PhysAddr::from_kernel_virtual(begin);
        let mut set = MemorySet::from((&elf, phys_start));
        let page_table = mc.make_page_table(&mut set);
        debug!("{:#x?}", set);

        use xmas_elf::header::HeaderPt2;
        let entry_addr = match elf.header.pt2 {
            HeaderPt2::Header64(header) => header.entry_point,
            _ => unimplemented!(),
        } as usize;

        let kstack = mc.alloc_stack(7).unwrap();
        let tf = TrapFrame::new_user_thread(entry_addr, kstack.top());
        let rsp = kstack.push_at_top(tf);

        Process {
            pid: 0,
            name: "user",
            kstack,
            status: Status::Ready,
            rsp,
            is_user: true,
        }
    }
}

use memory::{MemorySet, MemoryArea, PhysAddr, FromToVirtualAddress, EntryFlags};

impl<'a> From<(&'a ElfFile<'a>, PhysAddr)> for MemorySet {
    fn from(input: (&'a ElfFile<'a>, PhysAddr)) -> Self {
        let (elf, phys_start) = input;
        let mut set = MemorySet::new();
        for ph in elf.program_iter() {
            let ph = match ph {
                ProgramHeader::Ph64(ph) => ph,
                _ => unimplemented!(),
            };
            set.push(MemoryArea {
                start_addr: ph.virtual_addr as usize,
                end_addr: (ph.virtual_addr + ph.mem_size) as usize,
                phys_start_addr: Some(PhysAddr(phys_start.get() as u64 + ph.offset)),
                flags: EntryFlags::from(ph.flags).bits() as u32,
                name: "",
                mapped: false,
            });
        }
        set
    }
}

impl From<Flags> for EntryFlags {
    fn from(elf_flags: Flags) -> Self {
        let mut flags = EntryFlags::PRESENT;
        if elf_flags.is_write() {
            flags = flags | EntryFlags::WRITABLE;
        }
        if !elf_flags.is_execute() {
            flags = flags | EntryFlags::NO_EXECUTE;
        }
        flags
    }
}