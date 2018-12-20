/***************************************************************************}
{           PUBLIC USB 2.0 STRUCTURE DEFINITIONS AS PER THE MANUAL          }
{                     rpi-usb.h: line 127 ~ line 535                        }
****************************************************************************/
/*--------------------------------------------------------------------------}
{		Many parts of USB2.0 standard use this bit field for direction 	    }
{--------------------------------------------------------------------------*/
#[derive(PartialEq)]
#[repr(u32)]
pub enum UsbDirection {
    USB_DIRECTION_OUT = 0,											// Host to device
    USB_DIRECTION_IN = 1,											// Device to Host
}
/*--------------------------------------------------------------------------}
{	 Many parts of USB2.0 standard use this 2 bit field for speed control   }
{--------------------------------------------------------------------------*/
#[derive(PartialEq)]
#[repr(u32)]
pub enum UsbSpeed {
    USB_SPEED_HIGH = 0,												// USB high speed
    USB_SPEED_FULL = 1,												// USB full speed
    USB_SPEED_LOW = 2,												// USB low speed
}
/*--------------------------------------------------------------------------}
{			 Transfer types as layed out in USB 2.0 standard			    }
{--------------------------------------------------------------------------*/
#[derive(PartialEq)]
#[repr(u32)]
pub enum usb_transfer_type {
    USB_TRANSFER_TYPE_CONTROL = 0,
    USB_TRANSFER_TYPE_ISOCHRONOUS = 1,
    USB_TRANSFER_TYPE_BULK = 2,
    USB_TRANSFER_TYPE_INTERRUPT = 3,
}
/*--------------------------------------------------------------------------}
{			 Transfer sizes as layed out in USB 2.0 standard			    }
{--------------------------------------------------------------------------*/
#[derive(PartialEq)]
#[repr(u32)]
pub enum usb_transfer_size {
    USB_TRANSFER_SIZE_8_BIT = 0,
    USB_TRANSFER_SIZE_16_BIT = 1,
    USB_TRANSFER_SIZE_32_BIT = 2,
    USB_TRANSFER_SIZE_64_BIT = 3,
}
/*--------------------------------------------------------------------------}
{	 USB description types as per Table 9-5 in Section 9.4 of USB2.0 spec	}
{--------------------------------------------------------------------------*/
#[derive(PartialEq)]
#[repr(u32)]
pub enum usb_descriptor_type {
    USB_DESCRIPTOR_TYPE_DEVICE = 1,
    USB_DESCRIPTOR_TYPE_CONFIGURATION = 2,
    USB_DESCRIPTOR_TYPE_STRING = 3,
    USB_DESCRIPTOR_TYPE_INTERFACE = 4,
    USB_DESCRIPTOR_TYPE_ENDPOINT = 5,
    USB_DESCRIPTOR_TYPE_QUALIFIER = 6,
    USB_DESCRIPTOR_TYPE_OTHERSPEED_CONFIG = 7,
    USB_DESCRIPTOR_TYPE_INTERFACE_POWER = 8,
    USB_DESCRIPTOR_TYPE_HID = 33,
    USB_DESCRIPTOR_TYPE_HID_REPORT = 34,
    USB_DESCRIPTOR_TYPE_HID_PHYSICAL = 35,
    USB_DESCRIPTOR_TYPE_HUB = 41,
}
/*--------------------------------------------------------------------------}
{		 Enumeration Status defined in 9.1 of USB 2.0 standard			    }
{--------------------------------------------------------------------------*/
#[derive(PartialEq)]
#[repr(u32)]
pub enum UsbDeviceStatus {
    USB_STATUS_ATTACHED = 0,										// USB status is attached
    USB_STATUS_POWERED = 1,											// USB status is powered
    USB_STATUS_DEFAULT = 2,											// USB status is default
    USB_STATUS_ADDRESSED = 3,										// USB status is addressed
    USB_STATUS_CONFIGURED = 4,										// USB status is configured
}
/*--------------------------------------------------------------------------}
{		Hub Port Features that can be changed in the USB 2.0 standard	    }
{--------------------------------------------------------------------------*/
#[derive(PartialEq)]
#[repr(u32)]
pub enum HubPortFeature {
    FeatureConnection = 0,
    FeatureEnable = 1,
    FeatureSuspend = 2,
    FeatureOverCurrent = 3,
    FeatureReset = 4,
    FeaturePower = 8,
    FeatureLowSpeed = 9,
    FeatureHighSpeed = 10,
    FeatureConnectionChange = 16,
    FeatureEnableChange = 17,
    FeatureSuspendChange = 18,
    FeatureOverCurrentChange = 19,
    FeatureResetChange = 20,
}
/*--------------------------------------------------------------------------}
{		   Hub Gateway Node Features defined in the USB 2.0 standard		}
{--------------------------------------------------------------------------*/
#[derive(PartialEq)]
#[repr(u32)]
pub enum HubFeature {
    FeatureHubPower = 0,
    FeatureHubOverCurrent = 1,
}
/*--------------------------------------------------------------------------}
{	         USB class id as per 9.6.1 of USB2.0 manual enumerated			}
{--------------------------------------------------------------------------*/
#[derive(PartialEq)]
#[repr(u32)]
pub enum DeviceClass {
    DeviceClassInInterface = 0x00,
    DeviceClassCommunications = 0x2,
    DeviceClassHub = 0x9,
    DeviceClassDiagnostic = 0xdc,
    DeviceClassMiscellaneous = 0xef,
    DeviceClassVendorSpecific = 0xff,
}
/*--------------------------------------------------------------------------}
{	  Device Request structure (8 bytes) as per the USB 2.0 standard		}
{--------------------------------------------------------------------------*/
#[repr(C, packed)]
pub struct UsbDeviceRequest;
/*--------------------------------------------------------------------------}
{	         USB description header as per 9.6 of the USB2.0				}
{--------------------------------------------------------------------------*/
#[repr(C, packed)]
pub struct UsbDescriptorHeader {
    DescriptorLength: u8,										// +0x0
    DescriptorType: u8,											// +0x1
}
 /*--------------------------------------------------------------------------}
 {	   USB device descriptor .. Table 9-8 in 9.6.1 of the USB 2.0 spec	 	 }
 {--------------------------------------------------------------------------*/
#[repr(C, packed)]
pub struct usb_device_descriptor {
    bLength: u8,												// +0x0 Length of this descriptor
    bDescriptorType: u8,										// +0x1 Descriptor type
    bcdUSB: u16,												// +0x2 (in BCD 0x210 = USB2.10)
    bDeviceClass: u8,											// +0x4 Class code (enum DeviceClass )
    bDeviceSubClass: u8,										// +0x5 Subclass code (assigned by the USB-IF)
    bDeviceProtocol: u8,										// +0x6 Protocol code (assigned by the USB-IF)
    bMaxPacketSize0: u8,										// +0x7 Maximum packet size for endpoint 0
    idVendor: u16,												// +0x8 Vendor ID (assigned by the USB-IF)
    idProduct: u16,												// +0xa Product ID (assigned by the manufacturer)
    bcdDevice: u16,												// +0xc Device version number (BCD)
    iManufacturer: u8,											// +0xe Index of String Descriptor describing the manufacturer.
    iProduct: u8,												// +0xf Index of String Descriptor describing the product
    iSerialNumber: u8,											// +0x10 Index of String Descriptor with the device's serial number
    bNumConfigurations: u8,										// +0x11 Number of possible configurations
}
/*--------------------------------------------------------------------------}
{	  USB device configuration descriptor as per 9.6.3 of USB2.0 manual		}
{--------------------------------------------------------------------------*/
#[repr(C, packed)]
pub struct usb_configuration_descriptor;
/*--------------------------------------------------------------------------}
{  USB other speed configuration descriptor as per 9.6.4 of USB2.0 manual   }
{--------------------------------------------------------------------------*/
#[repr(C, packed)]
pub struct UsbOtherSpeedConfigurationDescriptor;
/*--------------------------------------------------------------------------}
{      USB interface descriptor structure as per 9.6.5 of USB2.0 manual     }
{--------------------------------------------------------------------------*/
#[derive(PartialEq)]
#[repr(u8)]
enum InterfaceClass {
    InterfaceClassReserved = 0x0,
    InterfaceClassAudio = 0x1,
    InterfaceClassCommunications = 0x2,
    InterfaceClassHid = 0x3,
    InterfaceClassPhysical = 0x5,
    InterfaceClassImage = 0x6,
    InterfaceClassPrinter = 0x7,
    InterfaceClassMassStorage = 0x8,
    InterfaceClassHub = 0x9,
    InterfaceClassCdcData = 0xa,
    InterfaceClassSmartCard = 0xb,
    InterfaceClassContentSecurity = 0xd,
    InterfaceClassVideo = 0xe,
    InterfaceClassPersonalHealthcare = 0xf,
    InterfaceClassAudioVideo = 0x10,
    InterfaceClassDiagnosticDevice = 0xdc,
    InterfaceClassWirelessController = 0xe0,
    InterfaceClassMiscellaneous = 0xef,
    InterfaceClassApplicationSpecific = 0xfe,
    InterfaceClassVendorSpecific = 0xff,
}
#[repr(C, packed)]
pub struct UsbInterfaceDescriptor {
    Header: UsbDescriptorHeader,    							// +0x0 Length of this descriptor, +0x1 DEVICE descriptor type (enum DescriptorType)
    Number: u8,													// +0x2 Number of this interface (0 based).
    AlternateSetting: u8,										// +0x3 Value of this alternate interface setting
    EndpointCount: u8,											// +0x4 Number of endpoints in this interface
    Class: InterfaceClass,										// +x05 Class code (assigned by the USB-IF)
    SubClass: u8,												// +x06 Subclass code (assigned by the USB-IF)
    Protocol: u8,												// +x07 Protocol code (assigned by the USB-IF)
    StringIndex: u8,											// +x08 Index of String Descriptor describing the interface
}
/*--------------------------------------------------------------------------}
{ USB endpoint descriptor structure (7 Bytes) as per 9.6.6 of USB2.0 manual }
{--------------------------------------------------------------------------*/
#[repr(C, packed)]
pub struct UsbEndpointDescriptor {
    Header: UsbDescriptorHeader,								// +0x0 Length of this descriptor, +0x1 DEVICE descriptor type (enum DescriptorType)
    #[repr(align(1))]
    EndpointAddress: u8,										// +0x2  Endpoint address. Bit 7 indicates direction (0=OUT, 1=IN).
    #[repr(align(1))]
    Attributes: u8,												// +0x3 Endpoint transfer type
    #[repr(align(1))]
    Packet: u16,												// +0x4 Maximum packet size.
    Interval: u8,												// +0x6 Polling interval in frames
}
/*--------------------------------------------------------------------------}
{       USB string descriptor structure as per 9.6.7 of USB2.0 manual       }
{--------------------------------------------------------------------------*/
#[repr(C, packed)]
pub struct UsbStringDescriptor;
/*--------------------------------------------------------------------------}
{ 	   USB HUB descriptor (9 Bytes) as per 11.23.2.1 of USB2.0 manual		}
{--------------------------------------------------------------------------*/
#[repr(C, packed)]
pub struct HubDescriptor;