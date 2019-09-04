//! mipsel drivers

use super::board;

pub use self::board::serial;

/// Initialize common drivers
pub fn init() {
    board::init_driver();
    crate::drivers::console::init();
}
