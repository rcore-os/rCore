//! mipsel drivers

use super::board;

pub use self::board::fb;
pub use self::board::serial;
#[path = "../../../drivers/console/mod.rs"]
pub mod console;

/// Initialize common drivers
pub fn init() {
    board::init_driver();
    console::init();
}
