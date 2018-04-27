use alloc::{boxed::Box, string::String, btree_map::BTreeMap};
use memory::{MemoryController, Stack};
use spin::{Once, Mutex};

/// 平台相关依赖：struct TrapFrame
///
/// ## 必须实现的特性
///
/// * Debug: 用于Debug输出
use arch::interrupt::TrapFrame;

pub fn init(mc: &mut MemoryController) {
    PROCESSOR.call_once(|| {Mutex::new({
        let mut processor = Processor::new(mc);
        let initproc = Process::new_init(mc);
        let idleproc = Process::new("idle", idle_thread, mc);
        processor.add(initproc);
        processor.add(idleproc);
        processor
    })});
}

static PROCESSOR: Once<Mutex<Processor>> = Once::new();

/// Called by timer handler in arch
/// 设置rsp，指向接下来要执行线程的 内核栈顶
/// 之后中断处理例程会重置rsp，恢复对应线程的上下文
pub fn schedule(rsp: &mut usize) {
    PROCESSOR.try().unwrap().lock().schedule(rsp);
}

#[derive(Debug)]
pub struct Process {
    pid: Pid,
    name: &'static str,
    kstack: Stack,
//    page_table: Box<PageTable>,
    status: Status,
    rsp: usize,
}

#[derive(Debug)]
enum Status {
    Uninit, Ready, Running, Sleeping(usize), Exited
}

struct Processor {
    procs: BTreeMap<Pid, Box<Process>>,
    current_pid: Pid,
}

type Pid = usize;

impl Processor {
    fn new(mc: &mut MemoryController) -> Self {
        Processor {
            procs: BTreeMap::<Pid, Box<Process>>::new(),
            current_pid: 0,
        }
    }
    fn alloc_pid(&self) -> Pid {
        let mut next: Pid = 0;
        for &i in self.procs.keys() {
            if i != next {
                return next;
            } else {
                next = i + 1;
            }
        }
        return next;
    }
    fn add(&mut self, mut process: Box<Process>) {
        let pid = self.alloc_pid();
        process.pid = pid;
        self.procs.insert(pid, process);
    }
    fn schedule(&mut self, rsp: &mut usize) {
        let pid = self.find_next();
        self.switch_to(pid, rsp);
    }
    fn find_next(&self) -> Pid {
        *self.procs.keys()
            .find(|&&i| i > self.current_pid)
            .unwrap_or(self.procs.keys().nth(0).unwrap())
    }
    fn switch_to(&mut self, pid: Pid, rsp: &mut usize) {
        // for debug print
        let pid0 = self.current_pid;
        let rsp0 = *rsp;

        if pid == self.current_pid {
            return;
        }
        {
            let current = self.procs.get_mut(&self.current_pid).unwrap();
            current.status = Status::Ready;
            current.rsp = *rsp;
        }
        {
            let process = self.procs.get_mut(&pid).unwrap();
            process.status = Status::Running;
            *rsp = process.rsp;
            // TODO switch page table
        }
        self.current_pid = pid;
        debug!("Processor: switch from {} to {}\n  rsp: {:#x} -> {:#x}", pid0, pid, rsp0, rsp);
    }
}

impl Process {
    /// Make a new kernel thread
    fn new(name: &'static str, entry: extern fn(), mc: &mut MemoryController) -> Box<Self> {
        let kstack = mc.alloc_stack(7).unwrap();
        let rsp = unsafe{ (kstack.top() as *mut TrapFrame).offset(-1) } as usize;

        let mut tf = unsafe{ &mut *(rsp as *mut TrapFrame) };

        // TODO: move to arch
        *tf = TrapFrame::default();
        tf.iret.cs = 8;
        tf.iret.rip = entry as usize;
        tf.iret.ss = 24;
        tf.iret.rsp = kstack.top();
        tf.iret.rflags = 0x282;

        Box::new(Process {
            pid: 0,
            name,
            kstack,
            status: Status::Ready,
            rsp,
        })
    }
    /// Make the first kernel thread `initproc`
    /// Should be called only once
    fn new_init(mc: &mut MemoryController) -> Box<Self> {
        assert_has_not_been_called!();
        Box::new(Process {
            pid: 0,
            name: "init",
            kstack: mc.kernel_stack.take().unwrap(),
            status: Status::Running,
            rsp: 0, // will be set at first schedule
        })
    }
}

extern fn idle_thread() {
    loop {
        println!("idle ...");
        let mut i = 0;
        while i < 1 << 22 {
            i += 1;
        }
    }
}