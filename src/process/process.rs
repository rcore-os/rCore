use super::*;
use memory::{Stack, InactivePageTable};
use xmas_elf::{ElfFile, program::{Flags, ProgramHeader}, header::HeaderPt2};
use core::slice;
use alloc::rc::Rc;
use rlibc::memcpy;

#[derive(Debug)]
pub struct Process {
    pub(in process) pid: Pid,
                    name: &'static str,
                    kstack: Stack,
    pub(in process) memory_set: Option<MemorySet>,
    pub(in process) page_table: Option<InactivePageTable>,
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
            memory_set: None,
            page_table: None,
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
            memory_set: None,
            page_table: None,
            status: Status::Running,
            rsp: 0, // will be set at first schedule
            is_user: false,
        }
    }

    pub fn new_user(begin: usize, end: usize, mc: &mut MemoryController) -> Self {
        // Parse elf
        let slice = unsafe{ slice::from_raw_parts(begin as *const u8, end - begin) };
        let elf = ElfFile::new(slice).expect("failed to read elf");

        // Make page table
        let mut memory_set = MemorySet::from(&elf);
        let page_table = mc.make_page_table(&mut memory_set);
        debug!("{:#x?}", memory_set);

        // Temporary switch to it, in order to copy data
        let page_table = mc.with(page_table, || {
            for ph in elf.program_iter() {
                let ph = match ph {
                    ProgramHeader::Ph64(ph) => ph,
                    _ => unimplemented!(),
                };
                unsafe { memcpy(ph.virtual_addr as *mut u8, (begin + ph.offset as usize) as *mut u8, ph.file_size as usize) };
            }
        });

        let entry_addr = match elf.header.pt2 {
            HeaderPt2::Header64(header) => header.entry_point,
            _ => unimplemented!(),
        } as usize;

        // Allocate kernel stack and push trap frame
        let kstack = mc.alloc_stack(7).unwrap();
        let tf = TrapFrame::new_user_thread(entry_addr, kstack.top());
        let rsp = kstack.push_at_top(tf);

        Process {
            pid: 0,
            name: "user",
            kstack,
            memory_set: Some(memory_set),
            page_table: Some(page_table),
            status: Status::Ready,
            rsp,
            is_user: true,
        }
    }
}

use memory::{MemorySet, MemoryArea, PhysAddr, FromToVirtualAddress, EntryFlags};

impl<'a> From<&'a ElfFile<'a>> for MemorySet {
    fn from(elf: &'a ElfFile<'a>) -> Self {
        let mut set = MemorySet::new();
        for ph in elf.program_iter() {
            let ph = match ph {
                ProgramHeader::Ph64(ph) => ph,
                _ => unimplemented!(),
            };
            set.push(MemoryArea {
                start_addr: ph.virtual_addr as usize,
                end_addr: (ph.virtual_addr + ph.mem_size) as usize,
                phys_start_addr: None,
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
        let mut flags = EntryFlags::PRESENT | EntryFlags::USER_ACCESSIBLE;
        if elf_flags.is_write() {
            flags = flags | EntryFlags::WRITABLE;
        }
        if !elf_flags.is_execute() {
            flags = flags | EntryFlags::NO_EXECUTE;
        }
        flags
    }
}