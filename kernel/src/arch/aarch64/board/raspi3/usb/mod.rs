mod c_structure;
mod c_structure_usb_2_0;
mod c_structure_usb_1_11;
mod c_api;

use self::c_structure::*;
use self::c_api::*;
pub use self::c_structure::{UsbDevice};
pub use self::c_api::{UsbShowTree};

pub fn init() {
    unsafe {
        check_size();
        UsbInitialise();
    }
}

pub fn get_root_hub() -> &'static mut UsbDevice {
    unsafe {
        &mut *UsbGetRootHub()
    }
}

#[no_mangle]
extern "C" fn rustos_print(s:*const u8) {
    use crate::util::from_cstr;
    print!("{}", unsafe{from_cstr(s)});
}
