use alloc::{boxed::Box, string::String, btree_map::BTreeMap};
use memory::{MemoryController, Stack};
use spin::{Once, Mutex};

/// 平台相关依赖：struct TrapFrame
///
/// ## 必须实现的特性
///
/// * Clone: 用于对栈中TrapFrame的替换
/// * Debug: 用于Debug输出
use arch::interrupt::TrapFrame;

pub fn init(mc: &mut MemoryController) {
    PROCESSOR.call_once(|| {Mutex::new(Processor::new(mc))});
}

static PROCESSOR: Once<Mutex<Processor>> = Once::new();

/// Called by timer handler in arch
pub fn schedule(trap_frame: &mut TrapFrame) {
    PROCESSOR.try().unwrap().lock().schedule(trap_frame);
}

#[derive(Debug)]
pub struct Process {
    pid: Pid,
    name: String,
    kstack: Stack,
//    page_table: Box<PageTable>,
    status: Status,
    trap_frame: TrapFrame,
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
        let mut processor = Processor {
            procs: BTreeMap::<Pid, Box<Process>>::new(),
            current_pid: 0,
        };
        let initproc = Box::new(Process{
            pid: 0,
            name: String::from("initproc"),
            kstack: mc.kernel_stack.take().unwrap(),
            status: Status::Running,
            trap_frame: TrapFrame::default(),
        });
        let idleproc = Box::new(Process{
            pid: 1,
            name: String::from("idleproc"),
            kstack: mc.alloc_stack(7).unwrap(),
            status: Status::Ready,
            trap_frame: {
                let mut tf = TrapFrame::default();
                tf.iret.cs = 8;
                tf.iret.rip = idle_thread as usize;
                tf.iret.rflags = 0x282;
                tf
            },
        });
        processor.procs.insert(0, initproc);
        processor.procs.insert(1, idleproc);
        processor
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
    fn schedule(&mut self, trap_frame: &mut TrapFrame) {
        self.switch(1, trap_frame);
    }
    fn switch(&mut self, pid: Pid, trap_frame: &mut TrapFrame) {
        if pid == self.current_pid {
            return;
        }
        {
            let current = self.procs.get_mut(&self.current_pid).unwrap();
            current.status = Status::Ready;
        }
        {
            let process = self.procs.get_mut(&pid).unwrap();
            *trap_frame = process.trap_frame.clone();
            // TODO switch page table
        }
        self.current_pid = pid;
    }
}

extern fn idle_thread() {
    loop {
        println!("idle ...");
        for i in 0 .. 1 << 20 {

        }
    }
}