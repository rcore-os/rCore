//! ARM64 drivers

use super::board;

/// Initialize ARM64 common drivers
pub fn init() {
    board::init_driver();
    crate::drivers::console::init();
}
