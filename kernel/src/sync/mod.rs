//! Useful synchronization primitives.
//!
//! 用于内核的同步互斥工具。
//!
//! 提供和`std::sync`相同的接口，具体用法可参考std官方文档。
//!
//! # 模块简介
//!
//! * `mutex`: 互斥锁。
//!     参考`spin::Mutex`实现了一套可替换底层支持的锁框架，在此基础上实现了三种锁：
//!     自旋锁，禁用中断自旋锁，线程调度锁
//!
//! * `condvar`: 条件变量。
//!     依赖`thread`，为其它工具提供线程调度支持。
//!
//! * `semaphore`: 信号量。
//!     完全照搬`std::sync::Semaphore`，std中已经废弃。
//!     貌似在Rust中并不常用，一般都用`Mutex`。
//!
//! * `mpsc`: 消息传递通道。
//!     多生产者-单消费者的FIFO队列。用于在线程间传递数据。
//!
//! * `test`: 测试。
//!     目前分别用`Mutex`和`Condvar`(Monitor)实现了哲学家就餐问题。
//!
//!
//! # 模块依赖关系图
//!
//! ```mermaid
//! graph TB
//!	subgraph dependence
//!	    interrupt
//!	    thread
//!	end
//!	subgraph sync
//!	    SpinLock --> interrupt
//!	    Condvar --> SpinLock
//!     Condvar --> thread
//!     Mutex --> Condvar
//!	    Monitor --> Condvar
//!	    Semaphore --> Condvar
//!	    Semaphore --> SpinLock
//!	    mpsc --> SpinLock
//!     mpsc --> Condvar
//!	end
//! subgraph test
//!	    Dining_Philosophers --> Mutex
//!	    Dining_Philosophers --> Monitor
//!	end
//! ```
#![allow(dead_code)]

pub use self::condvar::*;
pub use self::mutex::*;
pub use self::semaphore::*;

mod condvar;
pub mod mpsc;
mod mutex;
mod semaphore;
pub mod test;
