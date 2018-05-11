use super::*;
use memory::Stack;
use xmas_elf::ElfFile;
use core::slice;

#[derive(Debug)]
pub struct Process {
    pub(in process) pid: Pid,
                    name: &'static str,
                    kstack: Stack,
    //    page_table: Box<PageTable>,
    pub(in process) status: Status,
    pub(in process) rsp: usize,
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
        let rsp = unsafe{ (kstack.top() as *mut TrapFrame).offset(-1) } as usize;

        let tf = unsafe{ &mut *(rsp as *mut TrapFrame) };
        *tf = TrapFrame::new_kernel_thread(entry, kstack.top());

        Process {
            pid: 0,
            name,
            kstack,
            status: Status::Ready,
            rsp,
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
        }
    }

    pub fn new_user(begin: usize, end: usize, mc: &mut MemoryController) -> Self {
        let slice = unsafe{ slice::from_raw_parts(begin as *const u8, end - begin) };
        let elf = ElfFile::new(slice).expect("failed to read elf");
        for program_header in elf.program_iter() {
            println!("{:?}", program_header);
        }
        for section in elf.section_iter() {
            println!("{:?}", section);
        }
        unimplemented!();
    }
}

use memory::{MemorySet, MemoryArea};

fn new_memory_set_from_elf(elf: ElfFile, mc: &mut MemoryController) -> MemorySet {
    use xmas_elf::program::ProgramHeader;

    let mut set = MemorySet::new(mc);
    for ph in elf.program_iter() {
        match ph {
            ProgramHeader::Ph32(ph) => unimplemented!(),
            ProgramHeader::Ph64(ph) => {
                set.push(MemoryArea {
                    start_addr: ph.virtual_addr as usize,
                    end_addr: (ph.virtual_addr + ph.mem_size) as usize,
                    flags: ph.flags.0,  // TODO: handle it
                    name: "",
                    mapped: false,
                });
            },
        }
    }
    set
}