/***************************************************************************}
{          PUBLIC HID 1.11 STRUCTURE DEFINITIONS AS PER THE MANUAL          }
{                     rpi-usb.h: line 537 ~ line 645                        }
****************************************************************************/
/*--------------------------------------------------------------------------}
{ USB struct UsbDeviceRequest .Type Bit masks to use to make full bitmask   }
{--------------------------------------------------------------------------*/
const USB_SETUP_HOST_TO_DEVICE        : i32 = 0x00;    // Device Request bmRequestType transfer direction - host to device transfer
const USB_SETUP_DEVICE_TO_HOST        : i32 = 0x80;    // Device Request bmRequestType transfer direction - device to host transfer
const USB_SETUP_TYPE_STANDARD         : i32 = 0x00;    // Device Request bmRequestType type - standard
const USB_SETUP_TYPE_CLASS            : i32 = 0x20;    // Device Request bmRequestType type - class
const USB_SETUP_TYPE_VENDOR           : i32 = 0x40;    // Device Request bmRequestType type - vendor
const USB_SETUP_RECIPIENT_DEVICE      : i32 = 0x00;    // Device Request bmRequestType recipient - device
const USB_SETUP_RECIPIENT_INTERFACE   : i32 = 0x01;    // Device Request bmRequestType recipient - interface
const USB_SETUP_RECIPIENT_ENDPOINT    : i32 = 0x02;    // Device Request bmRequestType recipient - endpoint
const USB_SETUP_RECIPIENT_OTHER       : i32 = 0x03;    // Device Request bmRequestType recipient - other
/*--------------------------------------------------------------------------}
{ 		  USB struct UsbDeviceRequest .Type Bit masks for a HUB			    }
{--------------------------------------------------------------------------*/
const bmREQ_HUB_FEATURE             : i32 = USB_SETUP_HOST_TO_DEVICE | USB_SETUP_TYPE_CLASS | USB_SETUP_RECIPIENT_DEVICE;
const bmREQ_PORT_FEATURE            : i32 = USB_SETUP_HOST_TO_DEVICE | USB_SETUP_TYPE_CLASS | USB_SETUP_RECIPIENT_OTHER;
const bmREQ_HUB_STATUS              : i32 = USB_SETUP_DEVICE_TO_HOST | USB_SETUP_TYPE_CLASS | USB_SETUP_RECIPIENT_DEVICE;
const bmREQ_PORT_STATUS             : i32 = USB_SETUP_DEVICE_TO_HOST | USB_SETUP_TYPE_CLASS | USB_SETUP_RECIPIENT_OTHER;
const bmREQ_GET_HUB_DESCRIPTOR      : i32 = USB_SETUP_DEVICE_TO_HOST | USB_SETUP_TYPE_CLASS|USB_SETUP_RECIPIENT_DEVICE;
const bmREQ_SET_HUB_DESCRIPTOR      : i32 = USB_SETUP_HOST_TO_DEVICE | USB_SETUP_TYPE_CLASS|USB_SETUP_RECIPIENT_DEVICE;

const bmREQ_DEVICE_STATUS           : i32 = USB_SETUP_DEVICE_TO_HOST | USB_SETUP_TYPE_STANDARD | USB_SETUP_RECIPIENT_DEVICE;
const bmREQ_GET_DEVICE_DESCRIPTOR   : i32 = USB_SETUP_DEVICE_TO_HOST | USB_SETUP_TYPE_STANDARD | USB_SETUP_RECIPIENT_DEVICE;
const bmREQ_SET_DEVICE_DESCRIPTOR   : i32 = USB_SETUP_HOST_TO_DEVICE | USB_SETUP_TYPE_STANDARD | USB_SETUP_RECIPIENT_DEVICE;

const bmREQ_INTERFACE_FEATURE       : i32 = USB_SETUP_HOST_TO_DEVICE | USB_SETUP_TYPE_STANDARD | USB_SETUP_RECIPIENT_INTERFACE;
const bmREQ_INTERFACE_STATUS        : i32 = USB_SETUP_DEVICE_TO_HOST | USB_SETUP_TYPE_STANDARD | USB_SETUP_RECIPIENT_INTERFACE;

const bmREQ_ENDPOINT_FEATURE        : i32 = USB_SETUP_HOST_TO_DEVICE | USB_SETUP_TYPE_STANDARD | USB_SETUP_RECIPIENT_ENDPOINT;
const bmREQ_ENDPOINT_STATUS         : i32 = USB_SETUP_DEVICE_TO_HOST | USB_SETUP_TYPE_STANDARD | USB_SETUP_RECIPIENT_ENDPOINT;
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
#[repr(C, packed)]
struct HidDescriptor;