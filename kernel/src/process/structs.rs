use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, vec::Vec};
use core::fmt;

use log::*;
use rcore_fs::vfs::INode;
use spin::Mutex;
use xmas_elf::{ElfFile, header, program::{Flags, Type}};
use smoltcp::socket::{SocketSet, SocketHandle};
use smoltcp::wire::IpEndpoint;

use crate::arch::interrupt::{Context, TrapFrame};
use crate::memory::{ByFrame, GlobalFrameAlloc, KernelStack, MemoryAttr, MemorySet};
use crate::fs::{FileHandle, OpenOptions};
use crate::sync::Condvar;
use crate::drivers::NET_DRIVERS;
use crate::consts::{USER_TLS_OFFSET, USER_TMP_TLS_OFFSET};

use super::abi::{self, ProcInitInfo};

// TODO: avoid pub
pub struct Thread {
    pub context: Context,
    pub kstack: KernelStack,
    pub proc: Arc<Mutex<Process>>,
}

#[derive(Clone, Debug)]
pub enum SocketType {
    Raw,
    Tcp(Option<IpEndpoint>), // save local endpoint for bind()
    Udp,
    Icmp
}

#[derive(Clone, Debug)]
pub struct SocketWrapper {
    pub handle: SocketHandle,
    pub socket_type: SocketType,
}

#[derive(Clone)]
pub enum FileLike {
    File(FileHandle),
    Socket(SocketWrapper)
}

impl fmt::Debug for FileLike {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FileLike::File(_) => write!(f, "File"),
            FileLike::Socket(_) => write!(f, "Socket"),
        }
    }
}

pub struct Process {
    pub memory_set: MemorySet,
    pub files: BTreeMap<usize, FileLike>,
    pub cwd: String,
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
                cwd: String::from("/"),
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
                cwd: String::from("/"),
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
        let (mut memory_set, entry_addr, tls) = memory_set_from(&elf);

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
                    entry_addr, ustack_top, kstack.top(), is32, memory_set.token(), tls)
            },
            kstack,
            proc: Arc::new(Mutex::new(Process {
                memory_set,
                files,
                cwd: String::from("/"),
            })),
        })
    }

    /// Fork a new process from current one
    pub fn fork(&self, tf: &TrapFrame) -> Box<Thread> {
        info!("COME into fork!");
        // Clone memory set, make a new page table
        let memory_set = self.proc.lock().memory_set.clone();
        let files = self.proc.lock().files.clone();
        let cwd = self.proc.lock().cwd.clone();
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

        let iface = &*(NET_DRIVERS.read()[0]);
        let mut sockets = iface.sockets();
        for (_fd, file) in files.iter() {
            if let FileLike::Socket(wrapper) = file {
                sockets.retain(wrapper.handle);
            }
        }


        Box::new(Thread {
            context: unsafe { Context::new_fork(tf, kstack.top(), memory_set.token()) },
            kstack,
            proc: Arc::new(Mutex::new(Process {
                memory_set,
                files,
                cwd,
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
/// Also return the real entry point address and tls top addr.
fn memory_set_from(elf: &ElfFile<'_>) -> (MemorySet, usize, usize) {
    debug!("come in to memory_set_from");
    let mut ms = MemorySet::new();
    let mut entry = elf.header.pt2.entry_point() as usize;
    let mut tls = 0;

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
        if ph.get_type() != Ok(Type::Load) && ph.get_type() != Ok(Type::Tls) {
            continue;
        }

        let mut virt_addr = ph.virtual_addr() as usize;
        let offset = ph.offset() as usize;
        let file_size = ph.file_size() as usize;
        let mem_size = ph.mem_size() as usize;
        let mut name = "load";

        if ph.get_type() == Ok(Type::Tls) {
            virt_addr = USER_TLS_OFFSET;
            name = "tls";
            debug!("copying tls addr to {:X}", virt_addr);
        }

        #[cfg(target_arch = "aarch64")]
        assert_eq!((virt_addr >> 48), 0xffff, "Segment Fault");

        // Get target slice
        #[cfg(feature = "no_mmu")]
        let target = &mut target[virt_addr - va_begin..virt_addr - va_begin + mem_size];
        #[cfg(feature = "no_mmu")]
        info!("area @ {:?}, size = {:#x}", target.as_ptr(), mem_size);
        #[cfg(not(feature = "no_mmu"))]
        let target = {
            ms.push(virt_addr, virt_addr + mem_size, ByFrame::new(memory_attr_from(ph.flags()), GlobalFrameAlloc), &name);
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

        if ph.get_type() == Ok(Type::Tls) {
            virt_addr = USER_TMP_TLS_OFFSET;
            tls = virt_addr + ph.mem_size() as usize;
            debug!("copying tls addr to {:X}", virt_addr);

            // TODO: put this in a function
            // Get target slice
            #[cfg(feature = "no_mmu")]
            let target = &mut target[virt_addr - va_begin..virt_addr - va_begin + mem_size];
            #[cfg(feature = "no_mmu")]
            info!("area @ {:?}, size = {:#x}", target.as_ptr(), mem_size);
            #[cfg(not(feature = "no_mmu"))]
            let target = {
                ms.push(virt_addr, virt_addr + mem_size, ByFrame::new(memory_attr_from(ph.flags()).writable(), GlobalFrameAlloc), "tmptls");
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
    }
    (ms, entry, tls)
}

fn memory_attr_from(elf_flags: Flags) -> MemoryAttr {
    let mut flags = MemoryAttr::default().user();
    // TODO: handle readonly
    if elf_flags.is_execute() { flags = flags.execute(); }
    flags
}
