/// ARM64 drivers

use once::*;

use super::board;

/// Initialize ARM64 common drivers
pub fn init() {
    assert_has_not_been_called!();

    board::init_driver();
}
