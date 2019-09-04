//! ARM64 drivers

pub use self::board::serial;
use super::board;

/// Initialize ARM64 common drivers
pub fn init() {
    board::init_driver();
    crate::drivers::console::init();
}
