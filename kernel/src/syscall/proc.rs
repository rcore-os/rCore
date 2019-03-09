//! Syscalls for process

use super::*;

/// Fork the current process. Return the child's PID.
pub fn sys_fork(tf: &TrapFrame) -> SysResult {
    let new_thread = current_thread().fork(tf);
    let pid = processor().manager().add(new_thread, thread::current().id());
    info!("fork: {} -> {}", thread::current().id(), pid);
    Ok(pid)
}

/// Create a new thread in the current process.
/// The new thread's stack pointer will be set to `newsp`.
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
        // FIXME: see sys_mprotect
//        let proc = process();
//        proc.memory_set.check_mut_ptr(parent_tid)?;
//        proc.memory_set.check_mut_ptr(child_tid)?;
    }
    let new_thread = current_thread().clone(tf, newsp, newtls, child_tid as usize);
    // FIXME: parent pid
    let tid = processor().manager().add(new_thread, thread::current().id());
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
        let wait_procs = match target {
            WaitFor::AnyChild => processor().manager().get_children(thread::current().id()),
            WaitFor::Pid(pid) => {
                // check if pid is a child
                if processor().manager().get_children(thread::current().id()).iter()
                    .find(|&&p| p == pid).is_some() {
                    vec![pid]
                } else {
                    vec![]
                }
            }
        };
        if wait_procs.is_empty() {
            return Err(SysError::ECHILD);
        }

        for pid in wait_procs {
            match processor().manager().get_status(pid) {
                Some(Status::Exited(exit_code)) => {
                    if !wstatus.is_null() {
                        unsafe { wstatus.write(exit_code as i32); }
                    }
                    processor().manager().remove(pid);
                    info!("wait: {} -> {}", thread::current().id(), pid);
                    return Ok(pid);
                }
                None => return Err(SysError::ECHILD),
                _ => {}
            }
        }
        info!("wait: {} -> {:?}, sleep", thread::current().id(), target);
        match target {
            WaitFor::AnyChild => processor().manager().wait_child(thread::current().id()),
            WaitFor::Pid(pid) => processor().manager().wait(thread::current().id(), pid),
        }
        processor().yield_now();
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
    Ok(thread::current().id())
}

/// Get the current thread id
pub fn sys_gettid() -> SysResult {
    // use pid as tid for now
    Ok(thread::current().id())
}

/// Get the parent process id
pub fn sys_getppid() -> SysResult {
    let pid = thread::current().id();
    let ppid = processor().manager().get_parent(pid);
    Ok(ppid)
}

/// Exit the current thread
pub fn sys_exit(exit_code: isize) -> ! {
    let pid = thread::current().id();
    info!("exit: {}, code: {}", pid, exit_code);

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

    processor().manager().exit(pid, exit_code as usize);
    processor().yield_now();
    unreachable!();
}

pub fn sys_sleep(time: usize) -> SysResult {
    info!("sleep: time: {}", time);
    if time >= 1 << 31 {
        thread::park();
    } else {
        use core::time::Duration;
        thread::sleep(Duration::from_millis(time as u64 * 10));
    }
    Ok(0)
}

pub fn sys_set_priority(priority: usize) -> SysResult {
    let pid = thread::current().id();
    processor().manager().set_priority(pid, priority as u8);
    Ok(0)
}
