use alloc::{boxed::Box, collections::BTreeMap};
use scheduler::*;
use event_hub::EventHub;
use util::GetMut2;
use core::fmt::Debug;

#[derive(Debug)]
pub struct Process<T> {
    pid: Pid,
    parent: Pid,
    status: Status,
    context: T,
}

pub type Pid = usize;
pub type ErrorCode = usize;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Status {
    Ready,
    Running,
    Waiting(Pid),
    Sleeping,
    Exited(ErrorCode),
}

pub trait Context: Debug {
    unsafe fn switch(&mut self, target: &mut Self);
    fn new_kernel(entry: extern fn(usize) -> !, arg: usize) -> Self;
}

pub struct Processor_<T: Context, S: Scheduler> {
    procs: BTreeMap<Pid, Process<T>>,
    current_pid: Pid,
    event_hub: EventHub<Event>,
    /// Choose what on next schedule ?
    next: Option<Pid>,
    // WARNING: if MAX_PROCESS_NUM is too large, will cause stack overflow
    scheduler: S,
}

impl<T> Process<T> {
    fn exit_code(&self) -> Option<ErrorCode> {
        match self.status {
            Status::Exited(code) => Some(code),
            _ => None,
        }
    }
}

// TODO: 除schedule()外的其它函数，应该只设置进程状态，不应调用schedule
impl<T: Context, S: Scheduler> Processor_<T, S> {

    /*
    **  @brief  create a new Processor
    **  @param  init_context: T         initiate context
    **          scheduler: S            the scheduler to use
    **  @retval the Processor created
    */
    pub fn new(init_context: T, scheduler: S) -> Self {
        let init_proc = Process {
            pid: 0,
            parent: 0,
            status: Status::Running,
            context: init_context,
        };
        Processor_ {
            procs: {
                let mut map = BTreeMap::<Pid, Process<T>>::new();
                map.insert(0, init_proc);
                map
            },
            current_pid: 0,
            event_hub: EventHub::new(),
            next: None,
            scheduler,
        }
    }

    /*
    **  @brief  set the priority of current process
    **  @param  priority: u8            the priority to set
    **  @retval none
    */
    pub fn set_priority(&mut self, priority: u8) {
        self.scheduler.set_priority(self.current_pid, priority);
    }

    /*
    **  @brief  mark the current process to reschedule
    **  @param  none
    **  @retval none
    */
    pub fn set_reschedule(&mut self) {
        let pid = self.current_pid;
        self.set_status(pid, Status::Ready);
    }

    /*
    **  @brief  allocate the pid of the process
    **  @param  none
    **  @retval the pid allocated
    */
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

    /*
    **  @brief  set the status of the process
    **  @param  pid: Pid            the pid of process which needs to be set
    **          status: Status      the status to be set
    **  @retval none
    */
    fn set_status(&mut self, pid: Pid, status: Status) {
        let status0 = self.get(pid).status.clone();
        match (&status0, &status) {
            (&Status::Ready, &Status::Ready) => return,
            (&Status::Ready, _) => self.scheduler.remove(pid),
            (_, &Status::Ready) => self.scheduler.insert(pid),
            _ => {}
        }
        trace!("process {} {:?} -> {:?}", pid, status0, status);
        self.get_mut(pid).status = status;
    }

    /*
    **  @brief  Called by timer.
    **          Handle events.
    **  @param  none
    **  @retval none
    */
    pub fn tick(&mut self) {
        let current_pid = self.current_pid;
        if self.scheduler.tick(current_pid) {
            self.set_reschedule();
        }
        self.event_hub.tick();
        while let Some(event) = self.event_hub.pop() {
            debug!("event {:?}", event);
            match event {
                Event::Schedule => {
                    self.event_hub.push(10, Event::Schedule);
                    self.set_reschedule();
                },
                Event::Wakeup(pid) => {
                    self.set_status(pid, Status::Ready);
                    self.set_reschedule();
                    self.next = Some(pid);
                },
            }
        }
    }

    /*
    **  @brief  get now time
    **  @param  none
    **  @retval the time got
    */
    pub fn get_time(&self) -> usize {
        self.event_hub.get_time()
    }

    /*
    **  @brief  add a new process
    **  @param  context: T      the context fo the process
    **  @retval the pid of new process
    */
    pub fn add(&mut self, context: T) -> Pid {
        let pid = self.alloc_pid();
        let process = Process {
            pid,
            parent: self.current_pid,
            status: Status::Ready,
            context,
        };
        self.scheduler.insert(pid);
        self.procs.insert(pid, process);
        pid
    }

    /*
    **  @brief  Called every interrupt end
    **          Do schedule ONLY IF current status != Running
    **  @param  none
    **  @retval none
    */
    pub fn schedule(&mut self) {
        if self.get(self.current_pid).status == Status::Running {
            return;
        }
        let pid = self.next.take().unwrap_or_else(|| self.scheduler.select().unwrap());
        self.switch_to(pid);
    }

    /*
    **  @brief  Switch process to `pid`, switch page table if necessary.
                Store `rsp` and point it to target kernel stack.
                The current status must be set before, and not be `Running`.
    **  @param  the pid of the process to switch
    **  @retval none
    */
    fn switch_to(&mut self, pid: Pid) {
        // for debug print
        let pid0 = self.current_pid;

        if pid == self.current_pid {
            if self.get(self.current_pid).status != Status::Running {
                self.set_status(pid, Status::Running);
            }
            return;
        }
        self.current_pid = pid;

        let (from, to) = self.procs.get_mut2(pid0, pid);

        assert_ne!(from.status, Status::Running);
        assert_eq!(to.status, Status::Ready);
        to.status = Status::Running;
        self.scheduler.remove(pid);

        //info!("switch from {} to {} {:x?}", pid0, pid, to.context);
        unsafe { from.context.switch(&mut to.context); }
    }

    /*
    **  @brief  get process by pid
    **  @param  pid: Pid        the pid of the process
    **  @retval the process struct
    */
    fn get(&self, pid: Pid) -> &Process<T> {
        self.procs.get(&pid).unwrap()
    }

    /*
    **  @brief  get mut process struct by pid
    **  @param  pid: Pid        the pid of the process
    **  @retval the mut process struct
    */
    fn get_mut(&mut self, pid: Pid) -> &mut Process<T> {
        self.procs.get_mut(&pid).unwrap()
    }

    /*
    **  @brief  get context of current process
    **  @param  none
    **  @retval current context
    */
    pub fn current_context(&self) -> &T {
        &self.get(self.current_pid).context
    }

    pub fn current_context_mut(&mut self) -> &mut T {
        let id = self.current_pid;
        &mut self.get_mut(id).context
    }

    /*
    **  @brief  get pid of current process
    **  @param  none
    **  @retval current pid
    */
    pub fn current_pid(&self) -> Pid {
        self.current_pid
    }

    /*
    **  @brief  kill a process by pid
    **  @param  pid: Pid            the pid of the process to kill
    **  @retval none
    */
    pub fn kill(&mut self, pid: Pid) {
        self.exit(pid, 0x1000); // TODO: error code for killed
    }

    /*
    **  @brief  exit a process by pid
    **  @param  pid: Pid                    the pid to exit
    **          error_code: ErrorCode       the error code when exiting
    **  @retval none
    */
    pub fn exit(&mut self, pid: Pid, error_code: ErrorCode) {
        info!("{} exit, code: {}", pid, error_code);
        self.set_status(pid, Status::Exited(error_code));
        if let Some(waiter) = self.find_waiter(pid) {
            info!("  then wakeup {}", waiter);
            self.set_status(waiter, Status::Ready);
            self.next = Some(waiter);
        }
    }

    /*
    **  @brief  let a process to sleep for a while
    **  @param  pid: Pid            the pid of the process to sleep
    **          time: usize         the time to sleep
    **  @retval none
    */
    pub fn sleep(&mut self, pid: Pid, time: usize) {
        self.set_status(pid, Status::Sleeping);
        self.event_hub.push(time, Event::Wakeup(pid));
    }

    /*
    **  @brief  let a process to sleep until wake up
    **  @param  pid: Pid            the pid of the process to sleep
    **  @retval none
    */
    pub fn sleep_(&mut self, pid: Pid) {
        self.set_status(pid, Status::Sleeping);
    }

    /*
    **  @brief  wake up al sleeping process
    **  @param  pid: Pid            the pid of the process to wake up
    **  @retval none
    */
    pub fn wakeup_(&mut self, pid: Pid) {
        self.set_status(pid, Status::Ready);
    }

    /*
    **  @brief  Let current process wait for another
    **  @param  pid: Pid            the pid of the process to wait for
    **  @retval the result of wait
    */
    pub fn current_wait_for(&mut self, pid: Pid) -> WaitResult {
        info!("current {} wait for {:?}", self.current_pid, pid);
        if self.procs.values().filter(|&p| p.parent == self.current_pid).next().is_none() {
            return WaitResult::NotExist;
        }
        let pid = self.try_wait(pid).unwrap_or_else(|| {
            let current_pid = self.current_pid;
            self.set_status(current_pid, Status::Waiting(pid));
            self.schedule(); // yield
            self.try_wait(pid).unwrap()
        });
        let exit_code = self.get(pid).exit_code().unwrap();
        info!("{} wait end and remove {}", self.current_pid, pid);
        self.procs.remove(&pid);
        WaitResult::Ok(pid, exit_code)
    }

    /*
    **  @brief  Try to find a exited wait target
    **  @param  pid: Pid            the pid of the process to wait for
    **  @retval the pid found or none
    */
    fn try_wait(&mut self, pid: Pid) -> Option<Pid> {
        match pid {
            0 => self.procs.values()
                .find(|&p| p.parent == self.current_pid && p.exit_code().is_some())
                .map(|p| p.pid),
            _ => self.get(pid).exit_code().map(|_| pid),
        }
    }

    /*
    **  @brief  find one process which is waiting for the input process
    **  @param  pid: Pid            the pid of the target process
    **  @retval the pid of the waiting process or none
    */
    fn find_waiter(&self, pid: Pid) -> Option<Pid> {
        self.procs.values().find(|&p| {
            p.status == Status::Waiting(pid) ||
                (p.status == Status::Waiting(0) && self.get(pid).parent == p.pid)
        }).map(|ref p| p.pid)
    }
}

#[derive(Debug)]
pub enum WaitResult {
    /// The target process is exited with `ErrorCode`.
    Ok(Pid, ErrorCode),
    /// The target process is not exist.
    NotExist,
}

#[derive(Debug)]
enum Event {
    Schedule,
    Wakeup(Pid),
}

impl<T: Context> GetMut2<Pid> for BTreeMap<Pid, Process<T>> {
    type Output = Process<T>;
    fn get_mut(&mut self, id: Pid) -> &mut Process<T> {
        self.get_mut(&id).unwrap()
    }
}