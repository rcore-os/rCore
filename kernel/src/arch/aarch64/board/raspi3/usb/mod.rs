mod c_structure;
mod c_api;

use self::c_structure::*;
use self::c_api::*;

pub fn init() {
    unsafe {
        UsbInitialise();
    }
}

fn get_root_hub() -> UsbDevicePtr {
    unsafe { UsbGetRootHub() }
}
