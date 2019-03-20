#![no_std]
#![no_main]
#![feature(asm)]
#![feature(alloc)]
#![feature(naked_functions)]
#![feature(lang_items)]

extern crate alloc;

use core::alloc::Layout;
use core::panic::PanicInfo;
use alloc::{boxed::Box, sync::Arc};

use blog_os::{exit_qemu, gdt, interrupts::init_idt, serial_println};
use linked_list_allocator::LockedHeap;
use rcore_thread::{*, std_thread as thread};

const STACK_SIZE: usize = 0x2000;
const HEAP_SIZE: usize = 0x100000;
const MAX_CPU_NUM: usize = 1;
const MAX_PROC_NUM: usize = 32;


/// The entry of the kernel
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // init x86
    gdt::init();
    init_idt();
    // init log
    init_log();
    // init heap
    unsafe { HEAP_ALLOCATOR.lock().init(HEAP.as_ptr() as usize, HEAP_SIZE); }
    // init processor
    let scheduler = scheduler::RRScheduler::new(5);
    let thread_pool = Arc::new(ThreadPool::new(scheduler, MAX_PROC_NUM));
    unsafe { processor().init(0, Thread::init(), thread_pool); }
    // init threads
    thread::spawn(|| {
        let tid = processor().tid();
        serial_println!("[{}] yield", tid);
        thread::yield_now();
        serial_println!("[{}] spawn", tid);
        let t2 = thread::spawn(|| {
            let tid = processor().tid();
            serial_println!("[{}] yield", tid);
            thread::yield_now();
            serial_println!("[{}] return 8", tid);
            8
        });
        serial_println!("[{}] join", tid);
        let ret = t2.join();
        serial_println!("[{}] get {:?}", tid, ret);
        serial_println!("[{}] exit", tid);
    });
    // run threads
    processor().run();
}

fn init_log() {
    use log::*;
    struct SimpleLogger;
    impl Log for SimpleLogger {
        fn enabled(&self, _metadata: &Metadata) -> bool {
            true
        }
        fn log(&self, record: &Record) {
            serial_println!("[{:>5}] {}", record.level(), record.args());
        }
        fn flush(&self) {}
    }
    static LOGGER: SimpleLogger = SimpleLogger;
    set_logger(&LOGGER).unwrap();
    set_max_level(LevelFilter::Trace);
}

/// The context of a thread.
///
/// When a thread yield, its context will be stored at its stack.
#[derive(Debug, Default)]
#[repr(C)]
struct ContextData {
    rdi: usize, // arg0
    r15: usize,
    r14: usize,
    r13: usize,
    r12: usize,
    rbp: usize,
    rbx: usize,
    rip: usize,
}

impl ContextData {
    fn new(entry: extern fn(usize) -> !, arg0: usize) -> Self {
        ContextData {
            rip: entry as usize,
            rdi: arg0,
            ..ContextData::default()
        }
    }
}

#[repr(C)]
struct Thread {
    rsp: usize,
    stack: [u8; STACK_SIZE],
}

impl Thread {
    unsafe fn init() -> Box<Self> {
        Box::new(core::mem::uninitialized())
    }
    fn new(entry: extern fn(usize) -> !, arg0: usize) -> Box<Self> {
        let mut thread = unsafe { Thread::init() };
        let rsp = thread.stack.as_ptr() as usize + STACK_SIZE - core::mem::size_of::<ContextData>();
        // push a Context at stack top
        let init_context = ContextData::new(entry, arg0);
        unsafe { (rsp as *mut ContextData).write(init_context); }
        thread.rsp = rsp;
        thread
    }
}

/// Implement `switch_to` for a thread
impl Context for Thread {
    /// Switch to another thread.
    unsafe fn switch_to(&mut self, target: &mut Context) {
        let (to, _): (*mut Thread, usize) = core::mem::transmute(target);
        inner(self, to);

        #[naked]
        #[inline(never)]
        unsafe extern "C" fn inner(_from: *mut Thread, _to: *mut Thread) {
            asm!(
            "
            // push rip (by caller)

            // Save self callee-save registers
            push rbx
            push rbp
            push r12
            push r13
            push r14
            push r15
            push rdi

            // Switch stacks
            mov [rdi], rsp      // *rdi = from_rsp
            mov rsp, [rsi]      // *rsi = to_rsp

            // Restore target callee-save registers
            pop rdi
            pop r15
            pop r14
            pop r13
            pop r12
            pop rbp
            pop rbx

            // pop rip
            ret"
            : : : : "intel" "volatile" )
        }
    }

    fn set_tid(&mut self, _tid: usize) {
    }
}

/// Define global `Processor` for each core.
static PROCESSORS: [Processor; MAX_CPU_NUM] = [Processor::new()];

/// Now we only have one core.
fn cpu_id() -> usize { 0 }

/// Implement dependency for `rcore_thread::std_thread`
#[no_mangle]
pub fn processor() -> &'static Processor {
    &PROCESSORS[cpu_id()]
}

/// Implement dependency for `rcore_thread::std_thread`
#[no_mangle]
pub fn new_kernel_context(entry: extern fn(usize) -> !, arg0: usize) -> Box<Context> {
    Thread::new(entry, arg0)
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("\n{}", info);

    unsafe { exit_qemu(); }
    loop {}
}

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

#[lang = "oom"]
fn oom(_: Layout) -> ! {
    panic!("out of memory");
}