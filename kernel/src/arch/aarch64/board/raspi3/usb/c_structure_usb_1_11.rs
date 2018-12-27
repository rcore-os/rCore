/***************************************************************************}
{          PUBLIC HID 1.11 STRUCTURE DEFINITIONS AS PER THE MANUAL          }
{                     rpi-usb.h: line 537 ~ line 645                        }
****************************************************************************/
/*--------------------------------------------------------------------------}
{ USB struct UsbDeviceRequest .Type Bit masks to use to make full bitmask   }
{--------------------------------------------------------------------------*/
pub const USB_SETUP_HOST_TO_DEVICE        : i32 = 0x00;    // Device Request bmRequestType transfer direction - host to device transfer
pub const USB_SETUP_DEVICE_TO_HOST        : i32 = 0x80;    // Device Request bmRequestType transfer direction - device to host transfer
pub const USB_SETUP_TYPE_STANDARD         : i32 = 0x00;    // Device Request bmRequestType type - standard
pub const USB_SETUP_TYPE_CLASS            : i32 = 0x20;    // Device Request bmRequestType type - class
pub const USB_SETUP_TYPE_VENDOR           : i32 = 0x40;    // Device Request bmRequestType type - vendor
pub const USB_SETUP_RECIPIENT_DEVICE      : i32 = 0x00;    // Device Request bmRequestType recipient - device
pub const USB_SETUP_RECIPIENT_INTERFACE   : i32 = 0x01;    // Device Request bmRequestType recipient - interface
pub const USB_SETUP_RECIPIENT_ENDPOINT    : i32 = 0x02;    // Device Request bmRequestType recipient - endpoint
pub const USB_SETUP_RECIPIENT_OTHER       : i32 = 0x03;    // Device Request bmRequestType recipient - other
/*--------------------------------------------------------------------------}
{ 		  USB struct UsbDeviceRequest .Type Bit masks for a HUB			    }
{--------------------------------------------------------------------------*/
pub const bmREQ_HUB_FEATURE             : i32 = USB_SETUP_HOST_TO_DEVICE | USB_SETUP_TYPE_CLASS | USB_SETUP_RECIPIENT_DEVICE;
pub const bmREQ_PORT_FEATURE            : i32 = USB_SETUP_HOST_TO_DEVICE | USB_SETUP_TYPE_CLASS | USB_SETUP_RECIPIENT_OTHER;
pub const bmREQ_HUB_STATUS              : i32 = USB_SETUP_DEVICE_TO_HOST | USB_SETUP_TYPE_CLASS | USB_SETUP_RECIPIENT_DEVICE;
pub const bmREQ_PORT_STATUS             : i32 = USB_SETUP_DEVICE_TO_HOST | USB_SETUP_TYPE_CLASS | USB_SETUP_RECIPIENT_OTHER;
pub const bmREQ_GET_HUB_DESCRIPTOR      : i32 = USB_SETUP_DEVICE_TO_HOST | USB_SETUP_TYPE_CLASS|USB_SETUP_RECIPIENT_DEVICE;
pub const bmREQ_SET_HUB_DESCRIPTOR      : i32 = USB_SETUP_HOST_TO_DEVICE | USB_SETUP_TYPE_CLASS|USB_SETUP_RECIPIENT_DEVICE;

pub const bmREQ_DEVICE_STATUS           : i32 = USB_SETUP_DEVICE_TO_HOST | USB_SETUP_TYPE_STANDARD | USB_SETUP_RECIPIENT_DEVICE;
pub const bmREQ_GET_DEVICE_DESCRIPTOR   : i32 = USB_SETUP_DEVICE_TO_HOST | USB_SETUP_TYPE_STANDARD | USB_SETUP_RECIPIENT_DEVICE;
pub const bmREQ_SET_DEVICE_DESCRIPTOR   : i32 = USB_SETUP_HOST_TO_DEVICE | USB_SETUP_TYPE_STANDARD | USB_SETUP_RECIPIENT_DEVICE;

pub const bmREQ_INTERFACE_FEATURE       : i32 = USB_SETUP_HOST_TO_DEVICE | USB_SETUP_TYPE_STANDARD | USB_SETUP_RECIPIENT_INTERFACE;
pub const bmREQ_INTERFACE_STATUS        : i32 = USB_SETUP_DEVICE_TO_HOST | USB_SETUP_TYPE_STANDARD | USB_SETUP_RECIPIENT_INTERFACE;

pub const bmREQ_ENDPOINT_FEATURE        : i32 = USB_SETUP_HOST_TO_DEVICE | USB_SETUP_TYPE_STANDARD | USB_SETUP_RECIPIENT_ENDPOINT;
pub const bmREQ_ENDPOINT_STATUS         : i32 = USB_SETUP_DEVICE_TO_HOST | USB_SETUP_TYPE_STANDARD | USB_SETUP_RECIPIENT_ENDPOINT;
/*--------------------------------------------------------------------------}
{ 					 USB HID 1.11 defined report types						}
{--------------------------------------------------------------------------*/
#[derive(PartialEq)]
#[repr(u32)]
pub enum HidReportType {
    USB_HID_REPORT_TYPE_INPUT = 1,									// Input HID report
    USB_HID_REPORT_TYPE_OUTPUT = 2,									// Output HID report
    USB_HID_REPORT_TYPE_FEATURE = 3,								// Feature HID report
}
/*--------------------------------------------------------------------------}
{ 		 USB HID 1.11 descriptor structure as per manual in 6.2.1		    }
{--------------------------------------------------------------------------*/
#[repr(u8)]
pub enum HidCountry {
        CountryNotSupported = 0,
        Arabic = 1,
        Belgian = 2,
        CanadianBilingual = 3,
        CanadianFrench = 4,
        CzechRepublic = 5,
        Danish = 6,
        Finnish = 7,
        French = 8,
        German = 9,
        Greek = 10,
        Hebrew = 11,
        Hungary = 12,
        International = 13,
        Italian = 14,
        Japan = 15,
        Korean = 16,
        LatinAmerican = 17,
        Dutch = 18,
        Norwegian = 19,
        Persian = 20,
        Poland = 21,
        Portuguese = 22,
        Russian = 23,
        Slovakian = 24,
        Spanish = 25,
        Swedish = 26,
        SwissFrench = 27,
        SwissGerman = 28,
        Switzerland = 29,
        Taiwan = 30,
        TurkishQ = 31,
        EnglishUk = 32,
        EnglishUs = 33,
        Yugoslavian = 34,
        TurkishF = 35,
}
use super::c_structure_usb_2_0::{UsbDescriptorHeader};
#[repr(C, packed)]
pub struct HidDescriptor {
    pub Header: UsbDescriptorHeader,							// +0x0 Length of this descriptor, +0x1 DEVICE descriptor type (enum DescriptorType)
    pub HidVersion: u16, 										// (bcd version) +0x2
    pub Countrycode: HidCountry,								// +0x4
    pub DescriptorCount: u8,									// +0x5
    pub usb_descriptor_type: u8,								// +0x6
    pub Length: u16,											// +0x7
}
