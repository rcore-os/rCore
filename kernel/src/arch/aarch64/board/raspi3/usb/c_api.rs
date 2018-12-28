use super::c_structure::*;

extern "C" {
/***************************************************************************}
{                       PUBLIC INTERFACE ROUTINES                           }
{                     rpi-usb.h: line 761 ~ line 948                        }
****************************************************************************/

/*--------------------------------------------------------------------------}
{                        PUBLIC USB DESCRIPTOR ROUTINES                     }
{--------------------------------------------------------------------------*/
    pub fn HCDGetDescriptor (pipe: UsbPipe,                         // Pipe structure to send message thru (really just uint32_t)
                         type0: usb_descriptor_type,                // The type of descriptor
                         index: u8,                                 // The index of the type descriptor
                         langId: u16,                               // The language id
                         buffer: *mut u8,                           // Buffer to recieve descriptor
                         length: u32,                               // Maximumlength of descriptor
                         recipient: u8,                             // Recipient flags
                         bytesTransferred: *mut u32,                // Value at pointer will be updated with bytes transfered to/from buffer (NULL to ignore)
                         runHeaderCheck: bool,                      // Whether to run header check
    ) -> RESULT;
/*--------------------------------------------------------------------------}
{                    PUBLIC GENERIC USB INTERFACE ROUTINES                  }
{--------------------------------------------------------------------------*/
    pub fn UsbInitialise() -> RESULT;
    pub fn IsHub(devNumber: u8) -> bool;
    pub fn IsHid(devNumber: u8) -> bool;
    pub fn IsMassStorage(devNumber: u8) -> bool;
    pub fn IsMouse(devNumber: u8) -> bool;
    pub fn IsKeyboard(devNumber: u8) -> bool;
    pub fn UsbGetRootHub() -> *mut UsbDevice;
    pub fn UsbDeviceAtAddress(devNumber: u8) -> *mut UsbDevice;
/*--------------------------------------------------------------------------}
{                    PUBLIC USB CHANGE CHECKING ROUTINES                    }
{--------------------------------------------------------------------------*/
    pub fn UsbCheckForChange();
/*--------------------------------------------------------------------------}
{                    PUBLIC DISPLAY USB INTERFACE ROUTINES                  }
{--------------------------------------------------------------------------*/
    pub fn UsbGetDescription(device:*mut UsbDevice) -> *const u8;
    // fn UsbShowTree: obsolete, use UsbShowTree() below
/*--------------------------------------------------------------------------}
{                        PUBLIC HID INTERFACE ROUTINES                      }
{--------------------------------------------------------------------------*/
    pub fn HIDReadDescriptor(devNumber: u8,                         // Device number (address) of the device to read 
                              hidIndex: u8,                         // Which hid configuration information is requested from
                              Buffer: *mut u8,                      // Pointer to a buffer to receive the descriptor
                              Length: u16,                          // Maxium length of the buffer
    ) -> RESULT;
    pub fn HIDReadReport(devNumber: u8,                             // Device number (address) of the device to read
                          hidIndex: u8,                             // Which hid configuration information is requested from
                          reportValue: u16,                         // Hi byte = enum HidReportType  Lo Byte = Report Index (0 = default)  
                          Buffer: *mut u8,                          // Pointer to a buffer to recieve the report
                          Length: u16,                              // Length of the report
    ) -> RESULT;
    pub fn HIDWriteReport(devNumber: u8,                            // Device number (address) of the device to write report to
                           hidIndex: u8,                            // Which hid configuration information is writing to
                           reportValue: u16,                        // Hi byte = enum HidReportType  Lo Byte = Report Index (0 = default) 
                           Buffer: *mut u8,                         // Pointer to a buffer containing the report
                           Length: u16,                             // Length of the report
    ) -> RESULT;
    pub fn HIDSetProtocol(devNumber: u8,                            // Device number (address) of the device
                           interface: u8,                           // Interface number to change protocol on
                           protocol: u16,                           // The protocol number request
    ) -> RESULT;

/***************************************************************************}
{                       Other Interface for RustOS                          }
{                  usb-dependency.c: line 204 ~ line 217                    }
****************************************************************************/
    fn _RustOS_CheckSize(sizebuffer:*mut u32, length:u32) -> u32;
}

pub unsafe fn UsbShowTree(root:&mut UsbDevice, level:i32, tee:char) {
    use crate::util::from_cstr;
    const SpeedString:[&str;3] = [ "High", "Full", "Low" ];
    static mut TreeLevelInUse:[i32;20] = [0;20];

    let mut maxPacket:i32 = 0;
    for i in 0..(level-2) {
        if unsafe{TreeLevelInUse[i as usize]} == 0 {
            print!("   ")
        } else {
            print!(" | ")                    // Draw level lines if in use
        }
    }
    let maxPacket = SizeToNumber(root.Pipe0.getMaxSize());	                                // Max packet size
    println!(" {}-{} id: {} port: {} speed: {} packetsize: {} {}", tee,
        unsafe{from_cstr(UsbGetDescription(root))},
        root.Pipe0.getNumber(), root.ParentHub.PortNumber,
        SpeedString[root.Pipe0.getSpeed() as usize], maxPacket,
        if unsafe{IsHid(root.Pipe0.getNumber() as u8)} {"- HID interface"} else {""});	    // Print this entry
    if unsafe{IsHub(root.Pipe0.getNumber() as u8)} {
        let lastChild = unsafe{(*root.Payload.HubPayload).MaxChildren} as i32;
        for i in 0..lastChild-1 {	                                                        // For each child of hub
            let mut nodetee = '=';	                                                        // Preset nodetee to end node ... "L"
            for j in i..lastChild-2 {	                                                    // Check if any following child node is valid
                if !unsafe{(*root.Payload.HubPayload).Children[(j + 1)as usize]}.is_null() {// We found a following node in use                    
                    unsafe{TreeLevelInUse[level as usize] = 1;}	                            // Set tree level in use flag
                    nodetee = '+';	                                                        // Change the node character to tee looks like this "├"
                    break;	                                                                // Exit loop j
                }
            }
            if !unsafe{(*root.Payload.HubPayload).Children[i as usize]}.is_null() {	        // If child valid
                UsbShowTree(unsafe{&mut *(*root.Payload.HubPayload).Children[i as usize]},
                    level + 1, nodetee);	                                                // Iterate into child but level+1 down of coarse
            }
            unsafe{TreeLevelInUse[level as usize] = 0;}	                                    // Clear level in use flag
        }
    }
}

#[inline]
pub fn SizeToNumber(size: UsbPacketSize) -> u32 {
    use self::UsbPacketSize::*;
    match size {
        Bits8 => 8,
        Bits16 => 16,
        Bits32 => 32,
        _ => 64
    }
}

pub fn check_size() -> i32 {
    //basic checks
    let mut assert_failed: i32 = 0;
    macro_rules! Sizeof {
        ($type:ty) => (core::mem::size_of::<$type>() as u32);
    }
    macro_rules! AssertEq {
        ($e1:expr, $e2:expr; $($print_arg:expr),+) => {
            if $e1 != $e2 {
                assert_failed+=1;
                println!($($print_arg,)+)
            }
        };
    }
    macro_rules! AssertSize {
        ($type:ty, $size:expr, $msg:expr) => {
            if Sizeof!($type)!=0 {
                AssertEq!(Sizeof!($type), $size; "\tsizeof({})={}: {}", stringify!($type), Sizeof!($type), $msg)
            }
        };
    }
    //alignment checks
    let mut align_name: alloc::vec::Vec<&str> = alloc::vec::Vec::new();
    #[inline(always)]
    fn to_ptr<T>(r:&T) -> *const T { r as *const T }
    macro_rules! Align_Offset {
        ($struct:ty, $field:ident) => {{
            align_name.push(stringify!(&$struct::$field));
            to_ptr(unsafe{&(*(0 as *const $struct)).$field}) as u32
        }};
        ($struct:ty, Alternate($rust:ident, $_c:ident)) => {
            Align_Offset!($struct, $rust)
        };
    }
    macro_rules! Align_Sizeof {
        ($type:ty) => {{
            align_name.push(stringify!(sizeof($type)));
            Sizeof!($type)
        }};
    }
    println!("+check struct size");
    println!("\tcheck internal struct size");
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

/* OTHER STRUCTURES */
    println!("\tcheck value alignment between Rust and C");
    let size = vec![
        //pointer
        Align_Sizeof!(*const u8),
        Align_Sizeof!(*mut u8),
        //UsbDevice
        Align_Offset!(UsbDevice, ParentHub),
        Align_Offset!(UsbDevice, Pipe0),
        Align_Offset!(UsbDevice, PipeCtrl0),
        Align_Offset!(UsbDevice, Config),
        Align_Offset!(UsbDevice, MaxInterface),
        Align_Offset!(UsbDevice, Interfaces),
        Align_Offset!(UsbDevice, Endpoints),
        Align_Offset!(UsbDevice, Descriptor),
        Align_Offset!(UsbDevice, PayLoadId),
        Align_Offset!(UsbDevice, Alternate(Payload, HubPayload)),
        Align_Offset!(UsbDevice, Alternate(Payload, HidPayload)),
        Align_Offset!(UsbDevice, Alternate(Payload, MassPayload)),
        Align_Sizeof!(UsbParent),
        Align_Sizeof!(UsbPipe),
        Align_Sizeof!(UsbPipeControl),
        Align_Sizeof!(UsbConfigControl),
        Align_Sizeof!(UsbInterfaceDescriptor),
        Align_Sizeof!(UsbEndpointDescriptor),
        Align_Sizeof!(usb_device_descriptor),
        Align_Sizeof!(UsbDevice),
        //HubDevice
        Align_Offset!(HubDevice, MaxChildren),
        Align_Offset!(HubDevice, Children),
        Align_Offset!(HubDevice, Descriptor),
        Align_Sizeof!(HubDescriptor),
        Align_Sizeof!(HubDevice),
        //HidDevice
        Align_Offset!(HidDevice, Descriptor),
        Align_Offset!(HidDevice, HIDInterface),
        Align_Offset!(HidDevice, MaxHID),
        Align_Sizeof!(HidDescriptor),
        Align_Sizeof!(HidDevice),
        //MassStorageDevice
        Align_Offset!(MassStorageDevice, SCSI),
        Align_Sizeof!(MassStorageDevice),
    ];
    if size.len() != align_name.len() {
        AssertEq!(size.len(), align_name.len(); "\tsize.len() must equals others_name.len(), can't perform value alignment check")
    } else {
        let mut c_size = vec![0 as u32;size.len()];
        let ret = unsafe {
            _RustOS_CheckSize(c_size.as_mut_ptr(), c_size.len() as u32)
        };
        if ret!=0 {
            AssertEq!(size.len() as u32, ret; "\tplanned to check {} Rust alignment features, bug got {} C alignment features", size.len(), ret);
        } else {
            for i in 0..(size.len()-1) {
                AssertEq!(size[i], c_size[i]; "\t{}={}, which should be {}", align_name[i], size[i], c_size[i])
            }
        }
        if assert_failed == 0 {
            println!("-finish check struct size, succeed");
        } else {
            println!("-finish check struct size, total {} assertion failures", assert_failed);
        }
    }
    (-assert_failed)
}