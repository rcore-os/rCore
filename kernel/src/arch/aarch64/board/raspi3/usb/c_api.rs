use super::c_structure::*;

extern "C" {
	fn UsbInitialise() -> u32;
	fn UsbGetRootHub() -> UsbDevicePtr;
}
