use memory::MemoryController;
use spin::{Once, Mutex};
use core::slice;

use self::process::*;
use self::processor::*;

mod process;
mod processor;

/// 平台相关依赖：struct TrapFrame
///
/// ## 必须实现的特性
///
/// * Debug: 用于Debug输出
use arch::interrupt::TrapFrame;

// TODO: 使用宏来更优雅地导入符号，现在会有编译错误
//
//    #![feature(concat_idents)]
//
//    macro_rules! binary_symbol {
//        ($name: ident) => {
//            extern {
//                fn concat_idents!(_binary_user_, $name, _start)();
//                fn concat_idents!(_binary_user_, $name, _end)();
//            }
//        };
//    }
//
//    binary_symbol!(forktest);

#[cfg(feature = "link_user_program")]
extern {
    fn _binary_user_forktest_start();
    fn _binary_user_forktest_end();
    fn _binary_hello_start();
    fn _binary_hello_end();
}


pub fn init(mut mc: MemoryController) {
    PROCESSOR.call_once(|| {Mutex::new({
        let initproc = Process::new_init(&mut mc);
        let idleproc = Process::new("idle", idle_thread, &mut mc);
        #[cfg(feature = "link_user_program")]
//        let forktest = Process::new_user(_binary_user_forktest_start as usize,
//                                         _binary_user_forktest_end as usize, &mut mc);
        let hello = Process::new_user(_binary_hello_start as usize,
                                      _binary_hello_end as usize, &mut mc);
        let mut processor = Processor::new(mc);
        processor.add(initproc);
        processor.add(idleproc);
        processor.add(hello);
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

extern fn idle_thread() {
    loop {
        println!("idle ...");
        let mut i = 0;
        while i < 1 << 22 {
            i += 1;
        }
    }
}

/// Fork the current process
pub fn sys_fork(tf: &TrapFrame) -> i32 {
    PROCESSOR.try().unwrap().lock().fork(tf);
    0
}

/// Kill the process
pub fn sys_kill(pid: usize) -> i32 {
    PROCESSOR.try().unwrap().lock().kill(pid);
    0
}

/// Get the current process id
pub fn sys_getpid() -> i32 {
    PROCESSOR.try().unwrap().lock().current().pid as i32
}

/// Exit the current process
pub fn sys_exit(rsp: &mut usize, error_code: usize) -> i32 {
    let mut processor = PROCESSOR.try().unwrap().lock();
    let pid = processor.current().pid;
    processor.schedule(rsp);
    processor.exit(pid, error_code);
    0
}