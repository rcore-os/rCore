//! Device file system mounted at /dev

mod fbdev;
mod random;
mod serial;
mod shm;
mod tty;

pub use fbdev::*;
pub use random::*;
pub use serial::*;
pub use shm::*;
pub use tty::*;
