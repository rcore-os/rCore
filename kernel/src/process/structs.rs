use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, vec::Vec};

use log::*;
use rcore_fs::vfs::INode;
use spin::Mutex;
use xmas_elf::{ElfFile, header, program::{Flags, Type}};
use smoltcp::socket::{SocketSet, SocketHandle};

use crate::arch::interrupt::{Context, TrapFrame};
use crate::memory::{ByFrame, GlobalFrameAlloc, KernelStack, MemoryAttr, MemorySet};
use crate::fs::{FileHandle, OpenOptions};

use super::abi::{self, ProcInitInfo};

// TODO: avoid pub
pub struct Thread {
    pub context: Context,
    pub kstack: KernelStack,
    pub proc: Arc<Mutex<Process>>,
}

#[derive(Clone)]
pub enum FileLike {
    File(FileHandle),
    Socket(SocketHandle)
}

pub struct Process {
    pub memory_set: MemorySet,
    pub files: BTreeMap<usize, FileLike>,
    pub cwd: String,
    // TODO: discuss: move it to interface or leave it here
    pub sockets: SocketSet<'static, 'static, 'static>,
}

/// Let `rcore_thread` can switch between our `Thread`
impl rcore_thread::Context for Thread {
    unsafe fn switch_to(&mut self, target: &mut rcore_thread::Context) {
        use core::mem::transmute;
        let (target, _): (&mut Thread, *const ()) = transmute(target);
        self.context.switch(&mut target.context);
    }
}

impl Thread {
    /// Make a struct for the init thread
    pub unsafe fn new_init() -> Box<Thread> {
        Box::new(Thread {
            context: Context::null(),
            kstack: KernelStack::new(),
            proc: Arc::new(Mutex::new(Process {
                memory_set: MemorySet::new(),
                files: BTreeMap::default(),
                cwd: String::new(),
                sockets: SocketSet::new(vec![])
            })),
        })
    }

    /// Make a new kernel thread starting from `entry` with `arg`
    pub fn new_kernel(entry: extern fn(usize) -> !, arg: usize) -> Box<Thread> {
        let memory_set = MemorySet::new();
        let kstack = KernelStack::new();
        Box::new(Thread {
            context: unsafe { Context::new_kernel_thread(entry, arg, kstack.top(), memory_set.token()) },
            kstack,
            proc: Arc::new(Mutex::new(Process {
                memory_set,
                files: BTreeMap::default(),
                cwd: String::new(),
                sockets: SocketSet::new(vec![])
            })),
        })
    }

    /// Make a new user process from ELF `data`
    pub fn new_user<'a, Iter>(data: &[u8], args: Iter) -> Box<Thread>
        where Iter: Iterator<Item=&'a str>
    {
        // Parse elf
        let elf = ElfFile::new(data).expect("failed to read elf");
        let is32 = match elf.header.pt2 {
            header::HeaderPt2::Header32(_) => true,
            header::HeaderPt2::Header64(_) => false,
        };

        match elf.header.pt2.type_().as_type() {
            header::Type::Executable => {
//                #[cfg(feature = "no_mmu")]
//                panic!("ELF is not shared object");
            },
            header::Type::SharedObject => {},
            _ => panic!("ELF is not executable or shared object"),
        }

        // Make page table
        let (mut memory_set, entry_addr) = memory_set_from(&elf);

        // User stack
        use crate::consts::{USER_STACK_OFFSET, USER_STACK_SIZE, USER32_STACK_OFFSET};
        #[cfg(not(feature = "no_mmu"))]
        let mut ustack_top = {
            let (ustack_buttom, ustack_top) = match is32 {
                true => (USER32_STACK_OFFSET, USER32_STACK_OFFSET + USER_STACK_SIZE),
                false => (USER_STACK_OFFSET, USER_STACK_OFFSET + USER_STACK_SIZE),
            };
            memory_set.push(ustack_buttom, ustack_top,  ByFrame::new(MemoryAttr::default().user(), GlobalFrameAlloc), "user_stack");
            ustack_top
        };
        #[cfg(feature = "no_mmu")]
        let mut ustack_top = memory_set.push(USER_STACK_SIZE).as_ptr() as usize + USER_STACK_SIZE;

        let init_info = ProcInitInfo {
            args: args.map(|s| String::from(s)).collect(),
            envs: BTreeMap::new(),
            auxv: {
                let mut map = BTreeMap::new();
                if let Some(phdr) = elf.program_iter()
                    .find(|ph| ph.get_type() == Ok(Type::Phdr)) {
                    map.insert(abi::AT_PHDR, phdr.virtual_addr() as usize);
                }
                map.insert(abi::AT_PHENT, elf.header.pt2.ph_entry_size() as usize);
                map.insert(abi::AT_PHNUM, elf.header.pt2.ph_count() as usize);
                map
            },
        };
        unsafe {
            memory_set.with(|| { ustack_top = init_info.push_at(ustack_top) });
        }

        trace!("{:#x?}", memory_set);

        let kstack = KernelStack::new();

        let mut files = BTreeMap::new();
        files.insert(0, FileLike::File(FileHandle::new(crate::fs::STDIN.clone(), OpenOptions { read: true, write: false, append: false })));
        files.insert(1, FileLike::File(FileHandle::new(crate::fs::STDOUT.clone(), OpenOptions { read: false, write: true, append: false })));
        files.insert(2, FileLike::File(FileHandle::new(crate::fs::STDOUT.clone(), OpenOptions { read: false, write: true, append: false })));

        Box::new(Thread {
            context: unsafe {
                Context::new_user_thread(
                    entry_addr, ustack_top, kstack.top(), is32, memory_set.token())
            },
            kstack,
            proc: Arc::new(Mutex::new(Process {
                memory_set,
                files,
                cwd: String::new(),
                sockets: SocketSet::new(vec![])
            })),
        })
    }

    /// Fork a new process from current one
    pub fn fork(&self, tf: &TrapFrame) -> Box<Thread> {
        info!("COME into fork!");
        // Clone memory set, make a new page table
        let memory_set = self.proc.lock().memory_set.clone();
        info!("finish mmset clone in fork!");

        // MMU:   copy data to the new space
        // NoMMU: coping data has been done in `memory_set.clone()`
        #[cfg(not(feature = "no_mmu"))]
        for area in memory_set.iter() {
            let data = Vec::<u8>::from(unsafe { area.as_slice() });
            unsafe { memory_set.with(|| {
                area.as_slice_mut().copy_from_slice(data.as_slice())
            }) }
        }

        info!("temporary copy data!");
        let kstack = KernelStack::new();

        Box::new(Thread {
            context: unsafe { Context::new_fork(tf, kstack.top(), memory_set.token()) },
            kstack,
            proc: Arc::new(Mutex::new(Process {
                memory_set,
                files: self.proc.lock().files.clone(),
                cwd: String::new(),
                // TODO: duplicate sockets for child process
                sockets: SocketSet::new(vec![])
            })),
        })
    }
}

impl Process {
    pub fn get_free_inode(&self) -> usize {
        (0..).find(|i| !self.files.contains_key(i)).unwrap()
    }
}


/// Generate a MemorySet according to the ELF file.
/// Also return the real entry point address.
fn memory_set_from(elf: &ElfFile<'_>) -> (MemorySet, usize) {
    debug!("come in to memory_set_from");
    let mut ms = MemorySet::new();
    let mut entry = elf.header.pt2.entry_point() as usize;

    // [NoMMU] Get total memory size and alloc space
    let va_begin = elf.program_iter()
        .filter(|ph| ph.get_type() == Ok(Type::Load))
        .map(|ph| ph.virtual_addr()).min().unwrap() as usize;
    let va_end = elf.program_iter()
        .filter(|ph| ph.get_type() == Ok(Type::Load))
        .map(|ph| ph.virtual_addr() + ph.mem_size()).max().unwrap() as usize;
    let va_size = va_end - va_begin;
    #[cfg(feature = "no_mmu")]
    let target = ms.push(va_size);
    #[cfg(feature = "no_mmu")]
    { entry = entry - va_begin + target.as_ptr() as usize; }
    #[cfg(feature = "board_k210")]
    { entry += 0x40000000; }

    for ph in elf.program_iter() {
        if ph.get_type() != Ok(Type::Load) {
            continue;
        }
        let virt_addr = ph.virtual_addr() as usize;
        let offset = ph.offset() as usize;
        let file_size = ph.file_size() as usize;
        let mem_size = ph.mem_size() as usize;

        #[cfg(target_arch = "aarch64")]
        assert_eq!((virt_addr >> 48), 0xffff, "Segment Fault");

        // Get target slice
        #[cfg(feature = "no_mmu")]
        let target = &mut target[virt_addr - va_begin..virt_addr - va_begin + mem_size];
        #[cfg(feature = "no_mmu")]
        info!("area @ {:?}, size = {:#x}", target.as_ptr(), mem_size);
        #[cfg(not(feature = "no_mmu"))]
        let target = {
            ms.push(virt_addr, virt_addr + mem_size, ByFrame::new(memory_attr_from(ph.flags()), GlobalFrameAlloc), "");
            unsafe { ::core::slice::from_raw_parts_mut(virt_addr as *mut u8, mem_size) }
        };
        // Copy data
        unsafe {
            ms.with(|| {
                if file_size != 0 {
                    target[..file_size].copy_from_slice(&elf.input[offset..offset + file_size]);
                }
                target[file_size..].iter_mut().for_each(|x| *x = 0);
            });
        }
    }
    (ms, entry)
}

fn memory_attr_from(elf_flags: Flags) -> MemoryAttr {
    let mut flags = MemoryAttr::default().user();
    // TODO: handle readonly
    if elf_flags.is_execute() { flags = flags.execute(); }
    flags
}
