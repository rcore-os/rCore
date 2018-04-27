use memory::MemoryController;
use spin::{Once, Mutex};

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

extern fn idle_thread() {
    loop {
        println!("idle ...");
        let mut i = 0;
        while i < 1 << 22 {
            i += 1;
        }
    }
}