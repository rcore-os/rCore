//! mipsel drivers

use super::board;
use once::*;

pub use self::board::fb;
pub use self::board::serial;
#[path = "../../../drivers/console/mod.rs"]
pub mod console;

/// Initialize common drivers
pub fn init() {
    assert_has_not_been_called!("driver::init must be called only once");
    board::init_driver();
    console::init();
}
