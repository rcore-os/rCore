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
}


pub fn init(mc: &mut MemoryController) {
    PROCESSOR.call_once(|| {Mutex::new({
        let mut processor = Processor::new(mc);
        let initproc = Process::new_init(mc);
        let idleproc = Process::new("idle", idle_thread, mc);
        #[cfg(feature = "link_user_program")]
        let forktest = Process::new_user(_binary_user_forktest_start as usize,
                                         _binary_user_forktest_end as usize);
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