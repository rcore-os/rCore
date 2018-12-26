pub use super::c_structure_usb_1_11::*;
pub use super::c_structure_usb_2_0::*;
pub const MaximumDevices: u32 = 32;
pub const MaxChildrenPerDevice: u32 = 10;
pub const MaxInterfacesPerDevice: u32 = 8;
pub const MaxEndpointsPerDevice: u32 = 16;
pub const MaxHIDPerDevice: u32 = 4;
#[derive(PartialEq)]
#[repr(i32)]
pub enum RESULT {
    OK = 0,
    ErrorGeneral = -1,
    ErrorArgument = -2,
    ErrorRetry = -3,
    ErrorDevice = -4,
    ErrorIncompatible = -5,
    ErrorCompiler = -6,
    ErrorMemory = -7,
    ErrorTimeout = -8,
    ErrorHardware = -9,
    ErrorTransmission = -10,
    ErrorDisconnected = -11,
    ErrorDeviceNumber = -12,
    ErrorTooManyRetries = -13,
    ErrorIndex = -14,
    ErrorNotHID = -15,
    ErrorStall = -16,
}
#[derive(PartialEq)]
#[repr(u32)]
pub enum UsbPacketSize {
    Bits8 = 0,
    Bits16 = 1,
    Bits32 = 2,
    Bits64 = 3,
}

/***************************************************************************}
{             PUBLIC USB STRUCTURES DEFINITIONS DEFINED BY US				}
{                     rpi-usb.h: line 647 ~ line 760                        }
****************************************************************************/
/*--------------------------------------------------------------------------}
{	  To a standard USB device we can add a payload this is the type id		}
{--------------------------------------------------------------------------*/
#[derive(PartialEq)]
#[repr(u32)]
pub enum PayLoadType {
    ErrorPayload = 0,								// Device is not even active so can't have a payload							
    NoPayload = 1,									// Device is active but no payload attached
    HubPayload = 2,									// Device has hub payload attached
    HidPayload = 3,									// Device has Hid payload attached
    MassStoragePayload = 4,							// Device has Mass storage payload attached
}
/*--------------------------------------------------------------------------}
{ 	USB pipe our own special structure encompassing a pipe in the USB spec	}
{--------------------------------------------------------------------------*/
#[repr(C, packed)]
pub struct UsbPipe {
/*
    UsbPacketSize MaxSize : 2;										// @0		Maximum packet size
    UsbSpeed Speed : 2;												// @2		Speed of device
    unsigned EndPoint : 4;											// @4		Endpoint address
    unsigned Number : 8;											// @8		Unique device number sometimes called address or id
    unsigned _reserved : 2;											// @16-17
    unsigned lowSpeedNodePort : 7;									// @18-24		In low speed transfers it is port device is on closest parent high speed hub
    unsigned lowSpeedNodePoint : 7;									// @25-31	In low speed transfers it is closest parent high speed hub
*/
    pub rawfield : u32,
}
impl UsbPipe {
    const offset_MaxSize			:u32 =0;
    const offset_Speed				:u32 =2;
    const offset_EndPoint			:u32 =4;
    const offset_Number				:u32 =8;
    const offset_reserved			:u32 =16;
    const offset_lowSpeedNodePort	:u32 =18;
    const offset_lowSpeedNodePoint	:u32 =25;
    pub fn getMaxSize(&self) -> UsbPacketSize {
        use self::UsbPacketSize::*;
        match (self.rawfield>>UsbPipe::offset_MaxSize) & 0b11 {
            0 => Bits8,
            1 => Bits16,
            2 => Bits32,
            _ => Bits64,
        }
    }
    pub fn getSpeed(&self) -> UsbSpeed {
        use self::UsbSpeed::*;
        match (self.rawfield>>UsbPipe::offset_Speed) & 0b11 {
            0 => USB_SPEED_HIGH,
            1 => USB_SPEED_FULL,
            _ => USB_SPEED_LOW
        }
    }
    pub fn getNumber(&self) -> u32 {(self.rawfield>>UsbPipe::offset_Number) & 0xFF} //8 bit
}
/*--------------------------------------------------------------------------}
{ 			USB pipe control used mainly by internal routines				}
{--------------------------------------------------------------------------*/
#[repr(C, packed)]
pub struct UsbPipeControl {
/*
    unsigned _reserved : 14;										// @0-13	
    enum usb_transfer_type	Type : 2;								// @14-15	Packet type
    unsigned Channel : 8;											// @16-23   Channel to use
    unsigned Direction : 1;											// @24		Direction  1=IN, 0=OUT
    unsigned _reserved1 : 7;										// @25-31	
*/
    pub rawfield : u32,
}
/*--------------------------------------------------------------------------}
{ 	USB parent used mainly by internal routines (details of parent hub)		}
{--------------------------------------------------------------------------*/
#[repr(C, packed)]
pub struct UsbParent {
    pub Number : u8,											// @0	Unique device number of our parent sometimes called address or id
    pub PortNumber : u8,										// @8	This is the port we are connected to on our parent hub
    pub reserved : u16,											// @16  Reserved 16 bits
}
/*--------------------------------------------------------------------------}
{ 			USB config control used mainly by internal routines				}
{--------------------------------------------------------------------------*/
#[repr(C, packed)]
pub struct UsbConfigControl {
/*
    unsigned ConfigIndex : 8;										// @0 Current set config index
    unsigned ConfigStringIndex : 8;									// @8 Current config string index
    enum UsbDeviceStatus Status : 8;								// @16 Device enumeration status .. USB_ATTACHED, USB_POWERED, USB_ADDRESSED, etc
    unsigned reserved : 8;											// @24-31
*/
    pub rawfield : u32,
}
/*--------------------------------------------------------------------------}
{  Our structure that hold details about any USB device we have detected    }
{--------------------------------------------------------------------------*/
#[repr(C)]
pub union UsbDevice_Payload {					// It can only be any of the different payloads
    pub HubPayload: *mut HubDevice,				// If this is a USB gateway node of a hub this pointer will be set to the hub data which is about the ports
    pub HidPayload: *mut HidDevice,				// If this node has a HID function this pointer will be to the HID payload
    pub MassPayload: *mut MassStorageDevice,	// If this node has a MASS STORAGE function this pointer will be to the Mass Storage payload
}
#[repr(C)]
pub struct UsbDevice {
    pub ParentHub: UsbParent,						// Details of our parent hub
    pub Pipe0: UsbPipe,								// Usb device pipe AKA pipe0	
    pub PipeCtrl0: UsbPipeControl,					// Usb device pipe control AKA pipectrl0
    pub Config: UsbConfigControl,					// Usb config control
    #[repr(align(4))]
    pub MaxInterface: u8,							// Maxiumum interface in array (varies with config and usually a lot less than the max array size) 
    #[repr(align(4))]
    pub Interfaces: [UsbInterfaceDescriptor; MaxInterfacesPerDevice as usize], // These are available interfaces on this device
    #[repr(align(4))]
    pub Endpoints: [[UsbEndpointDescriptor; MaxInterfacesPerDevice as usize]; MaxEndpointsPerDevice as usize], // These are available endpoints on this device
    #[repr(align(4))]
    pub Descriptor: usb_device_descriptor,			// Device descriptor it's accessed a bit so we have a copy to save USB bus ... align it for ARM7/8
    pub PayLoadId: PayLoadType,						// Payload type being carried
    pub Payload: UsbDevice_Payload,
}
/*--------------------------------------------------------------------------}
{	 USB hub structure which is just extra data attached to a USB node	    }
{--------------------------------------------------------------------------*/
#[repr(C)]
pub struct HubDevice {
    pub MaxChildren: u32,
    pub Children: [*mut UsbDevice;MaxChildrenPerDevice as usize],
    #[repr(align(4))]
    pub Descriptor: HubDescriptor,				// Hub descriptor it's accessed a bit so we have a copy to save USB bus ... align it for ARM7/8
}
/*--------------------------------------------------------------------------}
{	 USB hid structure which is just extra data attached to a USB node	    }
{--------------------------------------------------------------------------*/
#[repr(C)]
pub struct HidDevice;
/*--------------------------------------------------------------------------}
{	USB mass storage structure which is extra data attached to a USB node   }
{--------------------------------------------------------------------------*/
#[repr(C)]
pub struct MassStorageDevice;
