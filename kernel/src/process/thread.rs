use super::{
    abi::{self, ProcInitInfo},
    Pid, Process,
};
use crate::arch::interrupt::TrapFrame;
use crate::arch::paging::*;
use crate::fs::{FileHandle, FileLike, OpenOptions, FOLLOW_MAX_DEPTH};
use crate::ipc::SemProc;
use crate::memory::{
    phys_to_virt, ByFrame, Delay, File, GlobalFrameAlloc, KernelStack, MemoryAttr, MemorySet, Read,
};
use crate::process::structs::ElfExt;
use crate::sync::{Condvar, SpinLock, SpinNoIrqLock as Mutex};
use crate::{
    signal::{Siginfo, Signal, SignalAction, SignalStack, Sigset},
    syscall::handle_syscall,
};
use alloc::{
    boxed::Box, collections::BTreeMap, collections::VecDeque, string::String, sync::Arc,
    sync::Weak, vec::Vec,
};
use apic::{LocalApic, XApic, LAPIC_ADDR};
use bitflags::_core::cell::Ref;
use core::fmt;
use core::str;
use core::{
    future::Future,
    mem::MaybeUninit,
    pin::Pin,
    task::{Context, Poll},
};
use log::*;
use pc_keyboard::KeyCode::BackTick;
use rcore_fs::vfs::INode;
use rcore_memory::{Page, PAGE_SIZE};
use spin::RwLock;
use trapframe::UserContext;
use x86_64::{
    registers::control::{Cr2, Cr3, Cr3Flags},
    structures::paging::PhysFrame,
    PhysAddr, VirtAddr,
};
use xmas_elf::{
    header,
    program::{Flags, SegmentData, Type},
    ElfFile,
};

/// Tid type
pub type Tid = usize;

/// Mutable part of a thread struct
#[derive(Default)]
struct ThreadInner {
    context: Option<Box<UserContext>>,
}

#[allow(dead_code)]
pub struct Thread {
    /// Mutable part
    inner: Mutex<ThreadInner>,
    /// Kernel stack
    kstack: KernelStack,
    /// Kernel performs futex wake when thread exits.
    /// Ref: [http://man7.org/linux/man-pages/man2/set_tid_address.2.html]
    pub clear_child_tid: usize,
    /// This is same as `proc.vm`, avoid extra locking
    pub vm: Arc<Mutex<MemorySet>>,
    /// The process that this thread belongs to
    pub proc: Arc<Mutex<Process>>,
    /// Thread id
    pub tid: Tid,
    /// Signal mask
    pub sig_mask: Sigset,
}

impl Thread {
    /// Construct virtual memory of a new user process from ELF at `inode`.
    /// Return `(MemorySet, entry_point, ustack_top)`
    pub fn new_user_vm(
        inode: &Arc<dyn INode>,
        args: Vec<String>,
        envs: Vec<String>,
    ) -> Result<(MemorySet, usize, usize), &'static str> {
        // Read ELF header
        // 0x3c0: magic number from ld-musl.so
        let mut data = [0u8; 0x3c0];
        inode
            .read_at(0, &mut data)
            .map_err(|_| "failed to read from INode")?;

        // Parse ELF
        let elf = ElfFile::new(&data)?;

        // Check ELF type
        match elf.header.pt2.type_().as_type() {
            header::Type::Executable => {}
            header::Type::SharedObject => {}
            _ => return Err("ELF is not executable or shared object"),
        }

        // Check ELF arch
        match elf.header.pt2.machine().as_machine() {
            #[cfg(target_arch = "x86_64")]
            header::Machine::X86_64 => {}
            #[cfg(target_arch = "aarch64")]
            header::Machine::AArch64 => {}
            #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
            header::Machine::Other(243) => {}
            #[cfg(target_arch = "mips")]
            header::Machine::Mips => {}
            _ => return Err("invalid ELF arch"),
        }

        // auxiliary vector
        let mut auxv = {
            let mut map = BTreeMap::new();
            if let Some(phdr_vaddr) = elf.get_phdr_vaddr() {
                map.insert(abi::AT_PHDR, phdr_vaddr as usize);
            }
            map.insert(abi::AT_PHENT, elf.header.pt2.ph_entry_size() as usize);
            map.insert(abi::AT_PHNUM, elf.header.pt2.ph_count() as usize);
            map.insert(abi::AT_PAGESZ, PAGE_SIZE);
            map
        };

        // entry point
        let mut entry_addr = elf.header.pt2.entry_point() as usize;
        // Make page table
        let (mut vm, bias) = elf.make_memory_set(inode);

        // Check interpreter (for dynamic link)
        // When interpreter is used, map both dynamic linker and executable
        if let Ok(loader_path) = elf.get_interpreter() {
            info!("Handling interpreter... offset={:x}", bias);
            // assuming absolute path
            let interp_inode = crate::fs::ROOT_INODE
                .lookup_follow(loader_path, FOLLOW_MAX_DEPTH)
                .map_err(|_| "interpreter not found")?;
            // load loader by bias and set aux vector.
            let mut interp_data: [u8; 0x3c0] = unsafe { MaybeUninit::zeroed().assume_init() };
            interp_inode
                .read_at(0, &mut interp_data)
                .map_err(|_| "failed to read from INode")?;
            let elf_interp = ElfFile::new(&interp_data)?;
            elf_interp.append_as_interpreter(&interp_inode, &mut vm, bias);

            // update auxiliary vector
            auxv.insert(abi::AT_ENTRY, elf.header.pt2.entry_point() as usize);
            auxv.insert(abi::AT_BASE, bias);

            // use interpreter as actual entry point
            debug!("entry point: {:x}", elf.header.pt2.entry_point() as usize);
            entry_addr = elf_interp.header.pt2.entry_point() as usize + bias;
        }

        // User stack
        use crate::consts::{USER_STACK_OFFSET, USER_STACK_SIZE};
        let mut ustack_top = {
            let ustack_buttom = USER_STACK_OFFSET;
            let ustack_top = USER_STACK_OFFSET + USER_STACK_SIZE;

            // user stack except top 4 pages
            vm.push(
                ustack_buttom,
                ustack_top - PAGE_SIZE * 4,
                MemoryAttr::default().user().execute(),
                Delay::new(GlobalFrameAlloc),
                "user_stack_delay",
            );

            // We are going to write init info now. So map the last 4 pages eagerly.
            vm.push(
                ustack_top - PAGE_SIZE * 4,
                ustack_top,
                MemoryAttr::default().user().execute(), // feature
                ByFrame::new(GlobalFrameAlloc),
                "user_stack",
            );
            ustack_top
        };

        // Make init info
        let init_info = ProcInitInfo { args, envs, auxv };
        unsafe {
            vm.with(|| ustack_top = init_info.push_at(ustack_top));
        }

        Ok((vm, entry_addr, ustack_top))
    }

    /// Make a new user process from ELF `data`
    pub fn new_user(
        inode: &Arc<dyn INode>,
        exec_path: &str,
        args: Vec<String>,
        envs: Vec<String>,
    ) -> Arc<Thread> {
        /// get virtual memory info
        let (vm, entry_addr, ustack_top) = Self::new_user_vm(inode, args, envs).unwrap();

        let vm_token = vm.token();
        let vm = Arc::new(Mutex::new(vm));
        let kstack = KernelStack::new();

        // initial fds
        let mut files = BTreeMap::new();
        files.insert(
            0,
            FileLike::File(FileHandle::new(
                crate::fs::STDIN.clone(),
                OpenOptions {
                    read: true,
                    write: false,
                    append: false,
                    nonblock: false,
                },
                String::from("stdin"),
                false,
                false,
            )),
        );
        files.insert(
            1,
            FileLike::File(FileHandle::new(
                crate::fs::STDOUT.clone(),
                OpenOptions {
                    read: false,
                    write: true,
                    append: false,
                    nonblock: false,
                },
                String::from("stdout"),
                false,
                false,
            )),
        );
        files.insert(
            2,
            FileLike::File(FileHandle::new(
                crate::fs::STDOUT.clone(),
                OpenOptions {
                    read: false,
                    write: true,
                    append: false,
                    nonblock: false,
                },
                String::from("stderr"),
                false,
                false,
            )),
        );

        // user context
        let mut context = UserContext::default();
        context.general.rip = entry_addr;
        context.general.rsp = ustack_top;
        context.general.rflags = 0x3202;

        Arc::new(Thread {
            tid: 1, // default is init
            inner: Mutex::new(ThreadInner {
                context: Some(Box::from(context)),
            }),
            kstack,
            clear_child_tid: 0,
            vm: vm.clone(),
            proc: Process {
                vm,
                files,
                cwd: String::from("/"),
                exec_path: String::from(exec_path),
                futexes: BTreeMap::default(),
                semaphores: SemProc::default(),
                pid: Pid::new(), // allocated in add_to_table()
                pgid: 0,
                parent: (Pid::new(), Weak::new()),
                children: Vec::new(),
                threads: Vec::new(),
                child_exit: Arc::new(Condvar::new()),
                child_exit_code: BTreeMap::new(),
                pending_sigset: Sigset::empty(),
                sig_queue: VecDeque::new(),
                dispositions: [SignalAction::default(); Signal::RTMAX + 1],
                sigaltstack: SignalStack::default(),
            }
            .add_to_table(),
            sig_mask: Sigset::default(),
        })
    }

    /// Fork a new process from current one
    /// Only current process is persisted
    pub fn fork(&self, tf: &UserContext) -> Box<Thread> {
        let kstack = KernelStack::new();
        /// clone virtual memory
        let vm = self.vm.lock().clone();
        let vm_token = vm.token();
        let vm = Arc::new(Mutex::new(vm));

        /// context of new thread
        let mut context = tf.clone();
        context.general.rax = 0;

        let mut proc = self.proc.lock();

        let new_proc = Process {
            vm: vm.clone(),
            files: proc.files.clone(), // share open file descriptions
            cwd: proc.cwd.clone(),
            exec_path: proc.exec_path.clone(),
            futexes: BTreeMap::default(),
            semaphores: proc.semaphores.clone(),
            pid: Pid::new(),
            pgid: proc.pgid,
            parent: (proc.pid.clone(), Arc::downgrade(&self.proc)),
            children: Vec::new(),
            threads: Vec::new(),
            child_exit: Arc::new(Condvar::new()),
            child_exit_code: BTreeMap::new(),
            pending_sigset: Sigset::empty(),
            sig_queue: VecDeque::new(),
            dispositions: proc.dispositions.clone(),
            sigaltstack: Default::default(),
        }
        .add_to_table();

        // link to parent
        let child_pid = new_proc.lock().pid.clone();
        proc.children.push((child_pid, Arc::downgrade(&new_proc)));

        // this part in linux manpage seems ambiguous:
        // Each of the threads in a process has its own signal mask.
        // A child created via fork(2) inherits a copy of its parent's signal
        // mask; the signal mask is preserved across execve(2).
        Box::new(Thread {
            tid: child_pid.get(), // tid = pid
            inner: Mutex::new(ThreadInner {
                context: Some(Box::new(context)),
            }),
            kstack,
            clear_child_tid: 0,
            vm,
            proc: new_proc,
            sig_mask: self.sig_mask,
        })
    }

    /// Create a new thread in the same process.
    pub fn clone(
        &self,
        tf: &TrapFrame,
        stack_top: usize,
        tls: usize,
        clear_child_tid: usize,
    ) -> Box<Thread> {
        let kstack = KernelStack::new();
        let vm_token = self.vm.lock().token();
        Box::new(Thread {
            tid: 0,
            inner: Mutex::new(ThreadInner::default()),
            kstack,
            clear_child_tid,
            vm: self.vm.clone(),
            proc: self.proc.clone(),
            sig_mask: self.sig_mask,
        })
    }

    pub fn begin_running(&self) -> Box<UserContext> {
        self.inner.lock().context.take().unwrap()
    }

    pub fn end_running(&self, cx: Box<UserContext>) {
        self.inner.lock().context = Some(cx);
    }
}
