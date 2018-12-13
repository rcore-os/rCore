use super::c_structure::*;

extern "C" {
	pub fn UsbInitialise() -> u32;
	pub fn UsbGetRootHub() -> UsbDevicePtr;
}
