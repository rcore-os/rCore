//! Syscalls for process

use super::*;
use crate::process::{PROCESSES, CHILD_PROCESSES};
use crate::sync::Condvar;

/// Fork the current process. Return the child's PID.
pub fn sys_fork(tf: &TrapFrame) -> SysResult {
    let new_thread = current_thread().fork(tf);
    let pid = processor().manager().add(new_thread);
    info!("fork: {} -> {}", thread::current().id(), pid);
    Ok(pid)
}

/// Create a new thread in the current process.
/// The new thread's stack pointer will be set to `newsp`,
///   and thread pointer will be set to `newtls`.
/// The child tid will be stored at both `parent_tid` and `child_tid`.
/// This is partially implemented for musl only.
pub fn sys_clone(flags: usize, newsp: usize, parent_tid: *mut u32, child_tid: *mut u32, newtls: usize, tf: &TrapFrame) -> SysResult {
    info!("clone: flags: {:#x}, newsp: {:#x}, parent_tid: {:?}, child_tid: {:?}, newtls: {:#x}",
        flags, newsp, parent_tid, child_tid, newtls);
    if flags != 0x7d0f00 {
        warn!("sys_clone only support musl pthread_create");
        return Err(SysError::ENOSYS);
    }
    {
        let proc = process();
        proc.memory_set.check_mut_ptr(parent_tid)?;
        proc.memory_set.check_mut_ptr(child_tid)?;
    }
    let new_thread = current_thread().clone(tf, newsp, newtls, child_tid as usize);
    // FIXME: parent pid
    let tid = processor().manager().add(new_thread);
    info!("clone: {} -> {}", thread::current().id(), tid);
    unsafe {
        parent_tid.write(tid as u32);
        child_tid.write(tid as u32);
    }
    Ok(tid)
}

/// Wait the process exit.
/// Return the PID. Store exit code to `code` if it's not null.
pub fn sys_wait4(pid: isize, wstatus: *mut i32) -> SysResult {
    info!("wait4: pid: {}, code: {:?}", pid, wstatus);
    let cur_pid = process().pid.get();
    if !wstatus.is_null() {
        process().memory_set.check_mut_ptr(wstatus)?;
    }
    #[derive(Debug)]
    enum WaitFor {
        AnyChild,
        Pid(usize),
    }
    let target = match pid {
        -1 => WaitFor::AnyChild,
        p if p > 0 => WaitFor::Pid(p as usize),
        _ => unimplemented!(),
    };
    loop {
        use alloc::vec;
        let all_child: Vec<_> = CHILD_PROCESSES.read().get(&cur_pid).unwrap().clone();
        let wait_procs = match target {
            WaitFor::AnyChild => all_child,
            WaitFor::Pid(pid) => {
                // check if pid is a child
                if let Some(proc) = all_child.iter().find(|p| p.lock().pid.get() == pid) {
                    vec![proc.clone()]
                } else {
                    vec![]
                }
            }
        };
        if wait_procs.is_empty() {
            return Err(SysError::ECHILD);
        }

        for proc_lock in wait_procs.iter() {
            let proc = proc_lock.lock();
            if let Some(exit_code) = proc.exit_code {
                // recycle process
                let pid = proc.pid.get();
                drop(proc);

                let mut child_processes = CHILD_PROCESSES.write();
                child_processes.get_mut(&cur_pid).unwrap().retain(|p| p.lock().pid.get() != pid);
                child_processes.remove(&pid);
                return Ok(pid);
            }
        }
        info!("wait: {} -> {:?}, sleep", thread::current().id(), target);

        for proc in wait_procs.iter() {
            proc.lock().exit_cond.add_to_wait_queue();
        }
        thread::park();
    }
}

pub fn sys_exec(name: *const u8, argv: *const *const u8, envp: *const *const u8, tf: &mut TrapFrame) -> SysResult {
    info!("exec: name: {:?}, argv: {:?} envp: {:?}", name, argv, envp);
    let proc = process();
    let name = if name.is_null() { String::from("") } else {
        unsafe { proc.memory_set.check_and_clone_cstr(name)? }
    };

    if argv.is_null() {
        return Err(SysError::EINVAL);
    }
    // Check and copy args to kernel
    let mut args = Vec::new();
    unsafe {
        let mut current_argv = argv as *const *const u8;
        while !(*current_argv).is_null() {
            let arg = proc.memory_set.check_and_clone_cstr(*current_argv)?;
            args.push(arg);
            current_argv = current_argv.add(1);
        }
    }
    info!("exec: args {:?}", args);

    // Read program file
    let path = args[0].as_str();
    let inode = crate::fs::ROOT_INODE.lookup(path)?;
    let size = inode.metadata()?.size;
    let mut buf = Vec::with_capacity(size);
    unsafe { buf.set_len(size); }
    inode.read_at(0, buf.as_mut_slice())?;

    // Make new Thread
    let iter = args.iter().map(|s| s.as_str());
    let mut thread = Thread::new_user(buf.as_slice(), iter);
    thread.proc.lock().files = proc.files.clone();
    thread.proc.lock().cwd = proc.cwd.clone();

    // Activate new page table
    unsafe { thread.proc.lock().memory_set.activate(); }

    // Modify the TrapFrame
    *tf = unsafe { thread.context.get_init_tf() };

    // Swap Context but keep KStack
    ::core::mem::swap(&mut current_thread().kstack, &mut thread.kstack);
    ::core::mem::swap(current_thread(), &mut *thread);

    Ok(0)
}

pub fn sys_yield() -> SysResult {
    thread::yield_now();
    Ok(0)
}

/// Kill the process
pub fn sys_kill(pid: usize) -> SysResult {
    info!("{} killed: {}", thread::current().id(), pid);
    processor().manager().exit(pid, 0x100);
    if pid == thread::current().id() {
        processor().yield_now();
    }
    Ok(0)
}

/// Get the current process id
pub fn sys_getpid() -> SysResult {
    info!("getpid");
    Ok(process().pid.get())
}

/// Get the current thread id
pub fn sys_gettid() -> SysResult {
    info!("gettid");
    // use pid as tid for now
    Ok(thread::current().id())
}

/// Get the parent process id
pub fn sys_getppid() -> SysResult {
    Ok(process().ppid.get())
}

/// Exit the current thread
pub fn sys_exit(exit_code: usize) -> ! {
    let tid = thread::current().id();
    info!("exit: {}, code: {}", tid, exit_code);
    let mut proc = process();
    proc.threads.retain(|&id| id != tid);
    if proc.threads.len() == 0 {
        // last thread
        proc.exit_code = Some(exit_code);
        proc.exit_cond.notify_all();
    }
    drop(proc);

    // perform futex wake 1
    // ref: http://man7.org/linux/man-pages/man2/set_tid_address.2.html
    // FIXME: do it in all possible ways a thread can exit
    //        it has memory access so we can't move it to Thread::drop?
    let clear_child_tid = current_thread().clear_child_tid;
    if clear_child_tid != 0 {
        unsafe { (clear_child_tid as *mut u32).write(0); }
        let queue = process().get_futex(clear_child_tid);
        queue.notify_one();
    }

    processor().manager().exit(tid, exit_code as usize);
    processor().yield_now();
    unreachable!();
}

/// Exit the current thread group (i.e. progress)
pub fn sys_exit_group(exit_code: usize) -> ! {
    let mut proc = process();
    info!("exit_group: {}, code: {}", proc.pid, exit_code);

    // quit all threads
    for tid in proc.threads.iter() {
        processor().manager().exit(*tid, exit_code);
    }
    proc.exit_code = Some(exit_code);
    proc.exit_cond.notify_all();
    drop(proc);

    processor().yield_now();
    unreachable!();
}

pub fn sys_nanosleep(req: *const TimeSpec) -> SysResult {
    process().memory_set.check_ptr(req)?;
    let time = unsafe { req.read() };
    info!("nanosleep: time: {:#?}", time);
    thread::sleep(time.to_duration());
    Ok(0)
}

pub fn sys_set_priority(priority: usize) -> SysResult {
    let pid = thread::current().id();
    processor().manager().set_priority(pid, priority as u8);
    Ok(0)
}
