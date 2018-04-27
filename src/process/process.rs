use super::*;
use memory::Stack;

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
}