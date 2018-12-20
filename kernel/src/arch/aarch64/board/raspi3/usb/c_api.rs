use super::c_structure::*;
use super::c_structure_usb_1_11::*;
use super::c_structure_usb_2_0::*;

extern "C" {
    pub fn UsbInitialise() -> u32;
    pub fn UsbGetRootHub() -> *mut UsbDevice;
}

#[inline]
fn SizeToNumber(size: UsbPacketSize) -> u32 {
    return if size == UsbPacketSize::Bits8 {8}
        else if size == UsbPacketSize::Bits16 {16}
        else if size == UsbPacketSize::Bits32 {32}
        else {64};
}
macro_rules! AssertSize {
    ($type:ty,$size:expr,$msg:expr) => {if core::mem::size_of::<$type>()!=($size) {print!($msg)}};
}
fn check_size() {
    println!("check struct size");
/* DESIGNWARE 2.0 REGISTERS */
//    AssertSize!(CoreOtgControl, 0x04, "Register/Structure should be 32bits (4 bytes)");
//    AssertSize!(CoreOtgInterrupt, 0x04, "Register/Structure should be 32bits (4 bytes)");
//    AssertSize!(CoreAhb, 0x04, "Register/Structure should be 32bits (4 bytes)");
//    AssertSize!(UsbControl, 0x04, "Register/Structure should be 32bits (4 bytes)");
//    AssertSize!(CoreReset, 0x04, "Register/Structure should be 32bits (4 bytes)");
//    AssertSize!(CoreInterrupts, 0x04, "Register/Structure should be 32bits (4 bytes)");
//    AssertSize!(CoreNonPeriodicInfo, 0x08, "Register/Structure should be 2x32bits (8 bytes)");
//    AssertSize!(CoreHardware, 0x10, "Register/Structure should be 4x32bits (16 bytes)");
//    AssertSize!(HostChannel, 0x20, "Register/Structure should be 8x32bits (32 bytes)");

/* USB SPECIFICATION STRUCTURES */
//    AssertSize!(HubPortFullStatus, 0x04, "Structure should be 32bits (4 bytes)");
//    AssertSize!(HubFullStatus, 0x04, "Structure should be 32bits (4 bytes)");
    AssertSize!(UsbDescriptorHeader, 0x02, "Structure should be 2 bytes");
    AssertSize!(UsbEndpointDescriptor, 0x07, "Structure should be 7 bytes");
    AssertSize!(UsbDeviceRequest, 0x08, "Structure should be 8 bytes");
    AssertSize!(HubDescriptor, 0x09, "Structure should be 9 bytes");
    AssertSize!(UsbInterfaceDescriptor, 0x09, "Structure should be 9 bytes");
    AssertSize!(usb_configuration_descriptor, 0x09, "Structure should be 9 bytes");
    AssertSize!(usb_device_descriptor, 0x12, "Structure should be 18 bytes");

/* INTERNAL STRUCTURES */
//    AssertSize!(UsbSendControl, 0x04, "Structure should be 32bits (4 bytes)");
    println!("finish check struct size");
}