use alloc::{boxed::Box, string::String, btree_map::BTreeMap};
use memory::Stack;

pub fn init(kstack: Stack) {
    let processor = Box::new(Processor::new(kstack));
    Box::into_raw(processor);
}

#[derive(Debug)]
pub struct Process {
    pid: Pid,
    name: String,
    kstack: Stack,
//    page_table: Box<PageTable>,
    status: Status,
    context: Context,
}

#[derive(Debug)]
struct Context {

}

#[derive(Debug)]
enum Status {
    Uninit, Ready, Running, Sleeping(usize), Exited
}

struct Processor {
    procs: BTreeMap<Pid, Box<Process>>,
}

type Pid = usize;

impl Processor {
    fn new(kernel_stack: Stack) -> Self {
        let mut processor = Processor {
            procs: BTreeMap::<Pid, Box<Process>>::new(),
        };
        let initproc = Box::new(Process{
            pid: 0,
            name: String::from("initproc"),
            kstack: kernel_stack,
            status: Status::Running,
            context: Context{},
        });
        processor.procs.insert(0, initproc);
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
    fn schedule(&self) {

    }
}