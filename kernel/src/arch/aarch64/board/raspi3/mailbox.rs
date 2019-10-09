//! Mailbox property interface
//!
//! (ref: https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface)

use crate::memory::kernel_offset;
use aarch64::cache::*;
use alloc::string::String;
use bcm2837::addr::phys_to_bus;
use bcm2837::mailbox::{Mailbox, MailboxChannel};
use core::mem;
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    static ref MAILBOX: Mutex<Mailbox> = Mutex::new(Mailbox::new());
}

#[derive(Debug)]
pub struct PropertyMailboxError(u32);
pub type PropertyMailboxResult<T> = Result<T, PropertyMailboxError>;

impl From<PropertyMailboxError> for String {
    fn from(error: PropertyMailboxError) -> Self {
        format!("{:x?}", error)
    }
}

/// Buffer request/response code.
/// Copied from `linux/include/soc/bcm2835/raspberrypi-firmware.h`
#[repr(u32)]
#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
#[allow(non_camel_case_types)]
enum PropertyMailboxStatus {
    RPI_FIRMWARE_STATUS_REQUEST = 0,
    RPI_FIRMWARE_STATUS_SUCCESS = 0x80000000,
    RPI_FIRMWARE_STATUS_ERROR = 0x80000001,
}
use self::PropertyMailboxStatus::*;

/// Tag identifier.
/// Copied from `linux/include/soc/bcm2835/raspberrypi-firmware.h`
#[repr(u32)]
#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
#[allow(non_camel_case_types)]
enum PropertyMailboxTagId {
    RPI_FIRMWARE_PROPERTY_END = 0,
    RPI_FIRMWARE_GET_FIRMWARE_REVISION = 0x00000001,

    RPI_FIRMWARE_SET_CURSOR_INFO = 0x00008010,
    RPI_FIRMWARE_SET_CURSOR_STATE = 0x00008011,

    RPI_FIRMWARE_GET_BOARD_MODEL = 0x00010001,
    RPI_FIRMWARE_GET_BOARD_REVISION = 0x00010002,
    RPI_FIRMWARE_GET_BOARD_MAC_ADDRESS = 0x00010003,
    RPI_FIRMWARE_GET_BOARD_SERIAL = 0x00010004,
    RPI_FIRMWARE_GET_ARM_MEMORY = 0x00010005,
    RPI_FIRMWARE_GET_VC_MEMORY = 0x00010006,
    RPI_FIRMWARE_GET_CLOCKS = 0x00010007,
    RPI_FIRMWARE_GET_POWER_STATE = 0x00020001,
    RPI_FIRMWARE_GET_TIMING = 0x00020002,
    RPI_FIRMWARE_SET_POWER_STATE = 0x00028001,
    RPI_FIRMWARE_GET_CLOCK_STATE = 0x00030001,
    RPI_FIRMWARE_GET_CLOCK_RATE = 0x00030002,
    RPI_FIRMWARE_GET_VOLTAGE = 0x00030003,
    RPI_FIRMWARE_GET_MAX_CLOCK_RATE = 0x00030004,
    RPI_FIRMWARE_GET_MAX_VOLTAGE = 0x00030005,
    RPI_FIRMWARE_GET_TEMPERATURE = 0x00030006,
    RPI_FIRMWARE_GET_MIN_CLOCK_RATE = 0x00030007,
    RPI_FIRMWARE_GET_MIN_VOLTAGE = 0x00030008,
    RPI_FIRMWARE_GET_TURBO = 0x00030009,
    RPI_FIRMWARE_GET_MAX_TEMPERATURE = 0x0003000a,
    RPI_FIRMWARE_GET_STC = 0x0003000b,
    RPI_FIRMWARE_ALLOCATE_MEMORY = 0x0003000c,
    RPI_FIRMWARE_LOCK_MEMORY = 0x0003000d,
    RPI_FIRMWARE_UNLOCK_MEMORY = 0x0003000e,
    RPI_FIRMWARE_RELEASE_MEMORY = 0x0003000f,
    RPI_FIRMWARE_EXECUTE_CODE = 0x00030010,
    RPI_FIRMWARE_EXECUTE_QPU = 0x00030011,
    RPI_FIRMWARE_SET_ENABLE_QPU = 0x00030012,
    RPI_FIRMWARE_GET_DISPMANX_RESOURCE_MEM_HANDLE = 0x00030014,
    RPI_FIRMWARE_GET_EDID_BLOCK = 0x00030020,
    RPI_FIRMWARE_GET_CUSTOMER_OTP = 0x00030021,
    RPI_FIRMWARE_GET_DOMAIN_STATE = 0x00030030,
    RPI_FIRMWARE_GET_THROTTLED = 0x00030046,
    RPI_FIRMWARE_GET_CLOCK_MEASURED = 0x00030047,
    RPI_FIRMWARE_NOTIFY_REBOOT = 0x00030048,
    RPI_FIRMWARE_SET_CLOCK_STATE = 0x00038001,
    RPI_FIRMWARE_SET_CLOCK_RATE = 0x00038002,
    RPI_FIRMWARE_SET_VOLTAGE = 0x00038003,
    RPI_FIRMWARE_SET_TURBO = 0x00038009,
    RPI_FIRMWARE_SET_CUSTOMER_OTP = 0x00038021,
    RPI_FIRMWARE_SET_DOMAIN_STATE = 0x00038030,
    RPI_FIRMWARE_GET_GPIO_STATE = 0x00030041,
    RPI_FIRMWARE_SET_GPIO_STATE = 0x00038041,
    RPI_FIRMWARE_SET_SDHOST_CLOCK = 0x00038042,
    RPI_FIRMWARE_GET_GPIO_CONFIG = 0x00030043,
    RPI_FIRMWARE_SET_GPIO_CONFIG = 0x00038043,
    RPI_FIRMWARE_GET_PERIPH_REG = 0x00030045,
    RPI_FIRMWARE_SET_PERIPH_REG = 0x00038045,
    RPI_FIRMWARE_GET_POE_HAT_VAL = 0x00030049,
    RPI_FIRMWARE_SET_POE_HAT_VAL = 0x00030050,

    /* Dispmanx TAGS */
    RPI_FIRMWARE_FRAMEBUFFER_ALLOCATE = 0x00040001,
    RPI_FIRMWARE_FRAMEBUFFER_BLANK = 0x00040002,
    RPI_FIRMWARE_FRAMEBUFFER_GET_PHYSICAL_WIDTH_HEIGHT = 0x00040003,
    RPI_FIRMWARE_FRAMEBUFFER_GET_VIRTUAL_WIDTH_HEIGHT = 0x00040004,
    RPI_FIRMWARE_FRAMEBUFFER_GET_DEPTH = 0x00040005,
    RPI_FIRMWARE_FRAMEBUFFER_GET_PIXEL_ORDER = 0x00040006,
    RPI_FIRMWARE_FRAMEBUFFER_GET_ALPHA_MODE = 0x00040007,
    RPI_FIRMWARE_FRAMEBUFFER_GET_PITCH = 0x00040008,
    RPI_FIRMWARE_FRAMEBUFFER_GET_VIRTUAL_OFFSET = 0x00040009,
    RPI_FIRMWARE_FRAMEBUFFER_GET_OVERSCAN = 0x0004000a,
    RPI_FIRMWARE_FRAMEBUFFER_GET_PALETTE = 0x0004000b,
    RPI_FIRMWARE_FRAMEBUFFER_GET_TOUCHBUF = 0x0004000f,
    RPI_FIRMWARE_FRAMEBUFFER_GET_GPIOVIRTBUF = 0x00040010,
    RPI_FIRMWARE_FRAMEBUFFER_RELEASE = 0x00048001,
    RPI_FIRMWARE_FRAMEBUFFER_TEST_PHYSICAL_WIDTH_HEIGHT = 0x00044003,
    RPI_FIRMWARE_FRAMEBUFFER_TEST_VIRTUAL_WIDTH_HEIGHT = 0x00044004,
    RPI_FIRMWARE_FRAMEBUFFER_TEST_DEPTH = 0x00044005,
    RPI_FIRMWARE_FRAMEBUFFER_TEST_PIXEL_ORDER = 0x00044006,
    RPI_FIRMWARE_FRAMEBUFFER_TEST_ALPHA_MODE = 0x00044007,
    RPI_FIRMWARE_FRAMEBUFFER_TEST_VIRTUAL_OFFSET = 0x00044009,
    RPI_FIRMWARE_FRAMEBUFFER_TEST_OVERSCAN = 0x0004400a,
    RPI_FIRMWARE_FRAMEBUFFER_TEST_PALETTE = 0x0004400b,
    RPI_FIRMWARE_FRAMEBUFFER_TEST_VSYNC = 0x0004400e,
    RPI_FIRMWARE_FRAMEBUFFER_SET_PHYSICAL_WIDTH_HEIGHT = 0x00048003,
    RPI_FIRMWARE_FRAMEBUFFER_SET_VIRTUAL_WIDTH_HEIGHT = 0x00048004,
    RPI_FIRMWARE_FRAMEBUFFER_SET_DEPTH = 0x00048005,
    RPI_FIRMWARE_FRAMEBUFFER_SET_PIXEL_ORDER = 0x00048006,
    RPI_FIRMWARE_FRAMEBUFFER_SET_ALPHA_MODE = 0x00048007,
    RPI_FIRMWARE_FRAMEBUFFER_SET_VIRTUAL_OFFSET = 0x00048009,
    RPI_FIRMWARE_FRAMEBUFFER_SET_OVERSCAN = 0x0004800a,
    RPI_FIRMWARE_FRAMEBUFFER_SET_PALETTE = 0x0004800b,
    RPI_FIRMWARE_FRAMEBUFFER_SET_TOUCHBUF = 0x0004801f,
    RPI_FIRMWARE_FRAMEBUFFER_SET_GPIOVIRTBUF = 0x00048020,
    RPI_FIRMWARE_FRAMEBUFFER_SET_VSYNC = 0x0004800e,
    RPI_FIRMWARE_FRAMEBUFFER_SET_BACKLIGHT = 0x0004800f,

    RPI_FIRMWARE_VCHIQ_INIT = 0x00048010,

    RPI_FIRMWARE_GET_COMMAND_LINE = 0x00050001,
    RPI_FIRMWARE_GET_DMA_CHANNELS = 0x00060001,
}
use self::PropertyMailboxTagId::*;

/// A property mailbox tag.
#[repr(C, packed)]
#[derive(Debug)]
#[allow(safe_packed_borrows)]
struct PropertyMailboxTag<T: Sized> {
    id: PropertyMailboxTagId,
    buf_size: u32,
    req_resp_size: u32,
    buf: T,
}

/// A request that contained a sequence of concatenated tags. The response
/// overwrites the request.
#[repr(C, packed)]
#[derive(Debug)]
#[allow(safe_packed_borrows)]
struct PropertyMailboxRequest<T: Sized> {
    buf_size: u32,
    req_resp_code: PropertyMailboxStatus,
    buf: T,
    end_tag: PropertyMailboxTagId,
}

/// Request buffer address must be 16-byte aligned.
#[repr(C, align(16))]
#[derive(Debug)]
struct Align16<T: Sized>(PropertyMailboxRequest<T>);

/// Some information of raspberry pi framebuffer
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RaspiFramebufferInfo {
    /// visible width
    pub xres: u32,
    /// visible height
    pub yres: u32,
    /// virtual width
    pub xres_virtual: u32,
    /// virtual height
    pub yres_virtual: u32,
    /// virtual offset x
    pub xoffset: u32,
    /// virtual offset y
    pub yoffset: u32,

    /// bits per pixel
    pub depth: u32,
    /// bytes per line
    pub pitch: u32,

    /// bus address, starts from 0xC0000000/0x40000000
    /// (see https://github.com/raspberrypi/firmware/wiki/Accessing-mailboxes)
    pub bus_addr: u32,
    /// screen buffer size
    pub screen_size: u32,
}

/// Pack a sequence of concatenated tags into a request, and send the address
/// to the mailbox.
/// Returns `PropertyMailboxResult<typeof($tags)>`.
macro_rules! send_request {
    ($tags: ident) => {{
        let req = Align16(PropertyMailboxRequest {
            buf_size: mem::size_of_val(&$tags) as u32 + 12,
            req_resp_code: RPI_FIRMWARE_STATUS_REQUEST,
            buf: $tags,
            end_tag: RPI_FIRMWARE_PROPERTY_END,
        });

        let start = &req as *const _ as usize;
        let end = start + req.0.buf_size as usize;
        {
            // flush data cache around mailbox accesses
            let mut mbox = MAILBOX.lock();
            DCache::<Clean, PoC>::flush_range(start, end, SY);
            mbox.write(
                MailboxChannel::Property,
                phys_to_bus(kernel_offset(start) as u32),
            );
            mbox.read(MailboxChannel::Property);
            DCache::<Invalidate, PoC>::flush_range(start, end, SY);
        }

        match req.0.req_resp_code {
            RPI_FIRMWARE_STATUS_SUCCESS => Ok(req.0.buf),
            other => Err(PropertyMailboxError(other as u32)),
        }
    }};
}

/// Send a tag to mailbox. Will call `send_request!`.
/// Returns `PropertyMailboxResult<typeof(buf)>`.
macro_rules! send_one_tag {
    ($id: expr, [$($arg: expr),*]) => {{
        let buf = [$($arg),*];
        let tag = PropertyMailboxTag {
            id: $id,
            buf_size: mem::size_of_val(&buf) as u32,
            req_resp_size: 0,
            buf,
        };
        Ok(send_request!(tag)?.buf)
    }};
}

/// Allocates contiguous memory on the GPU. `size` and `align` are in bytes.
/// Returns memory `handle`.
pub fn mem_alloc(size: u32, align: u32, flags: u32) -> PropertyMailboxResult<u32> {
    let ret = send_one_tag!(RPI_FIRMWARE_LOCK_MEMORY, [size, align, flags])?;
    Ok(ret[0])
}

/// Free the memory buffer of `handle`. status=0 is success.
pub fn mem_free(handle: u32) -> PropertyMailboxResult<()> {
    let status = send_one_tag!(RPI_FIRMWARE_RELEASE_MEMORY, [handle])?;
    match status[0] {
        0 => Ok(()),
        other => Err(PropertyMailboxError(other)),
    }
}

/// Lock buffer in place, and return a `bus_address`. Must be done before memory
/// can be accessed.
pub fn mem_lock(handle: u32) -> PropertyMailboxResult<u32> {
    let ret = send_one_tag!(RPI_FIRMWARE_LOCK_MEMORY, [handle])?;
    Ok(ret[0])
}

/// Unlock buffer. It retains contents, but may move. Needs to be locked before
/// next use. status=0 is success.
pub fn mem_unlock(handle: u32) -> PropertyMailboxResult<()> {
    let status = send_one_tag!(RPI_FIRMWARE_UNLOCK_MEMORY, [handle])?;
    match status[0] {
        0 => Ok(()),
        other => Err(PropertyMailboxError(other)),
    }
}

/// Get physical (display) width/height. Returns `(width, height)` in pixels.
/// Note that the "physical (display)" size is the size of the allocated buffer
/// in memory, not the resolution of the video signal sent to the display device.
pub fn framebuffer_get_physical_size() -> PropertyMailboxResult<(u32, u32)> {
    let ret = send_one_tag!(RPI_FIRMWARE_FRAMEBUFFER_GET_PHYSICAL_WIDTH_HEIGHT, [0, 0])?;
    Ok((ret[0], ret[1]))
}

/// Get depth. Returns bits per pixel.
pub fn framebuffer_get_depth() -> PropertyMailboxResult<u32> {
    let ret = send_one_tag!(RPI_FIRMWARE_FRAMEBUFFER_GET_DEPTH, [0])?;
    Ok(ret[0])
}

/// Set virtual offset. Returns `(X, Y)` in pixel.
/// The response may not be the same as the request so it must be checked.
/// May be the previous offset or 0 for unsupported.
pub fn framebuffer_set_virtual_offset(
    xoffset: u32,
    yoffset: u32,
) -> PropertyMailboxResult<(u32, u32)> {
    let ret = send_one_tag!(
        RPI_FIRMWARE_FRAMEBUFFER_SET_VIRTUAL_OFFSET,
        [xoffset, yoffset]
    )?;
    Ok((ret[0], ret[1]))
}

/// Allocate framebuffer on GPU and try to set width/height/depth.
/// Returns `RaspiFramebufferInfo`.
pub fn framebuffer_alloc(
    width: u32,
    height: u32,
    depth: u32,
) -> PropertyMailboxResult<RaspiFramebufferInfo> {
    #[repr(C, packed)]
    #[derive(Debug)]
    struct FramebufferAllocTag {
        set_physical_size: PropertyMailboxTag<[u32; 2]>,
        set_virtual_size: PropertyMailboxTag<[u32; 2]>,
        set_depth: PropertyMailboxTag<[u32; 1]>,
        set_virtual_offset: PropertyMailboxTag<[u32; 2]>,
        allocate: PropertyMailboxTag<[u32; 2]>,
        get_pitch: PropertyMailboxTag<[u32; 1]>,
    }

    let tags = FramebufferAllocTag {
        // Set physical (buffer) width/height. Returns `(width, height)` in pixel.
        set_physical_size: PropertyMailboxTag {
            id: RPI_FIRMWARE_FRAMEBUFFER_SET_PHYSICAL_WIDTH_HEIGHT,
            buf_size: 8,
            req_resp_size: 0,
            buf: [width, height],
        },
        // Set virtual (buffer) width/height. Returns `(width, height)` in pixel.
        set_virtual_size: PropertyMailboxTag {
            id: RPI_FIRMWARE_FRAMEBUFFER_SET_VIRTUAL_WIDTH_HEIGHT,
            buf_size: 8,
            req_resp_size: 0,
            buf: [width, height],
        },
        // Set depth; Returns bits per pixel.
        set_depth: PropertyMailboxTag {
            id: RPI_FIRMWARE_FRAMEBUFFER_SET_DEPTH,
            buf_size: 4,
            req_resp_size: 0,
            buf: [depth],
        },
        // Set virtual offset. Returns `(X, Y)` in pixel.
        set_virtual_offset: PropertyMailboxTag {
            id: RPI_FIRMWARE_FRAMEBUFFER_SET_VIRTUAL_OFFSET,
            buf_size: 8,
            req_resp_size: 0,
            buf: [0, 0],
        },
        // Allocate buffer. Returns `(base_address, size)` in bytes.
        allocate: PropertyMailboxTag {
            id: RPI_FIRMWARE_FRAMEBUFFER_ALLOCATE,
            buf_size: 8,
            req_resp_size: 0,
            buf: [0x1000, 0],
        },
        // Get pitch. Return bytes per line.
        get_pitch: PropertyMailboxTag {
            id: RPI_FIRMWARE_FRAMEBUFFER_GET_PITCH,
            buf_size: 4,
            req_resp_size: 0,
            buf: [0],
        },
    };

    let ret = send_request!(tags)?;
    Ok(RaspiFramebufferInfo {
        xres: ret.set_physical_size.buf[0],
        yres: ret.set_physical_size.buf[1],
        xres_virtual: ret.set_virtual_size.buf[0],
        yres_virtual: ret.set_virtual_size.buf[1],
        xoffset: ret.set_virtual_offset.buf[0],
        yoffset: ret.set_virtual_offset.buf[1],

        depth: ret.set_depth.buf[0],
        pitch: ret.get_pitch.buf[0],

        bus_addr: ret.allocate.buf[0],
        screen_size: ret.allocate.buf[1],
    })
}

pub fn get_clock_rate(clock_id: u32) -> PropertyMailboxResult<u32> {
    let ret = send_one_tag!(RPI_FIRMWARE_GET_CLOCK_RATE, [clock_id, 0])?;
    if ret[0] == clock_id {
        return Ok(ret[1]);
    } else {
        return Err(PropertyMailboxError(1));
    }
}
