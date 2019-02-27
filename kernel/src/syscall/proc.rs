//! Syscalls for process

use super::*;

/// Fork the current process. Return the child's PID.
pub fn sys_fork(tf: &TrapFrame) -> SysResult {
    let context = current_thread().fork(tf);
    let pid = processor().manager().add(context, thread::current().id());
    info!("fork: {} -> {}", thread::current().id(), pid);
    Ok(pid as isize)
}

/// Wait the process exit.
/// Return the PID. Store exit code to `code` if it's not null.
pub fn sys_wait(pid: usize, code: *mut i32) -> SysResult {
    // TODO: check ptr
    loop {
        use alloc::vec;
        let wait_procs = match pid {
            0 => processor().manager().get_children(thread::current().id()),
            _ => vec![pid],
        };
        if wait_procs.is_empty() {
            return Ok(-1);
        }
        for pid in wait_procs {
            match processor().manager().get_status(pid) {
                Some(Status::Exited(exit_code)) => {
                    if !code.is_null() {
                        unsafe { code.write(exit_code as i32); }
                    }
                    processor().manager().remove(pid);
                    info!("wait: {} -> {}", thread::current().id(), pid);
                    return Ok(0);
                }
                None => return Ok(-1),
                _ => {}
            }
        }
        info!("wait: {} -> {}, sleep", thread::current().id(), pid);
        if pid == 0 {
            processor().manager().wait_child(thread::current().id());
            processor().yield_now();
        } else {
            processor().manager().wait(thread::current().id(), pid);
            processor().yield_now();
        }
    }
}

pub fn sys_exec(name: *const u8, argc: usize, argv: *const *const u8, tf: &mut TrapFrame) -> SysResult {
    // TODO: check ptr
    let name = if name.is_null() { "" } else { unsafe { util::from_cstr(name) } };
    info!("exec: {:?}, argc: {}, argv: {:?}", name, argc, argv);
    // Copy args to kernel
    let args: Vec<String> = unsafe {
        slice::from_raw_parts(argv, argc).iter()
            .map(|&arg| String::from(util::from_cstr(arg)))
            .collect()
    };

    if args.len() <= 0 {
        return Err(SysError::Inval);
    }
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
    Ok(thread::current().id() as isize)
}

/// Exit the current process
pub fn sys_exit(exit_code: isize) -> ! {
    let pid = thread::current().id();
    info!("exit: {}, code: {}", pid, exit_code);
    processor().manager().exit(pid, exit_code as usize);
    processor().yield_now();
    unreachable!();
}

pub fn sys_sleep(time: usize) -> SysResult {
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
