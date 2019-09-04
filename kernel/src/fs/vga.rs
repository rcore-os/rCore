use rcore_fs::vfs::*;

use crate::drivers::gpu::fb::FRAME_BUFFER;
use crate::memory::phys_to_virt;
use alloc::{string::String, sync::Arc, vec::Vec};
use core::any::Any;

#[derive(Default)]
pub struct Vga;

impl INode for Vga {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        Err(FsError::NotSupported)
    }
    fn write_at(&self, _offset: usize, _buf: &[u8]) -> Result<usize> {
        info!("the _offset is {} {}", _offset, _buf[0]);
        let lock = FRAME_BUFFER.lock();
        if let Some(ref frame_buffer) = *lock {
            use core::slice;
            let frame_buffer_data = unsafe {
                slice::from_raw_parts_mut(
                    frame_buffer.base_addr() as *mut u8,
                    frame_buffer.framebuffer_size(),
                )
            };
            frame_buffer_data.copy_from_slice(&_buf);
            Ok(frame_buffer.framebuffer_size())
        } else {
            Err(FsError::EntryNotFound)
        }
    }
    fn poll(&self) -> Result<PollStatus> {
        Ok(PollStatus {
            // TOKNOW and TODO
            read: true,
            write: false,
            error: false,
        })
    }
    fn metadata(&self) -> Result<Metadata> {
        Ok(Metadata {
            dev: 0,
            inode: 0,
            size: 0x24000,
            blk_size: 0,
            blocks: 0,
            atime: Timespec { sec: 0, nsec: 0 },
            mtime: Timespec { sec: 0, nsec: 0 },
            ctime: Timespec { sec: 0, nsec: 0 },
            type_: FileType::SymLink,
            mode: 0,
            nlinks: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
        })
    }
    fn io_control(&self, cmd: u32, data: usize) -> Result<()> {
        info!("cmd {:#x} , data {:#x} vga not support ioctl !", cmd, data);
        match cmd {
            FBIOGET_FSCREENINFO => {
                let fb_fix_info = unsafe { &mut *(data as *mut fb_fix_screeninfo) };
                if let Some(fb) = FRAME_BUFFER.lock().as_ref() {
                    fb.fill_fix_screeninfo(fb_fix_info);
                }
                Ok(())
            }
            FBIOGET_VSCREENINFO => {
                let fb_var_info = unsafe { &mut *(data as *mut fb_var_screeninfo) };
                if let Some(fb) = FRAME_BUFFER.lock().as_ref() {
                    fb.fill_var_screeninfo(fb_var_info);
                }
                Ok(())
            }
            _ => {
                warn!("use never support ioctl !");
                Err(FsError::NotSupported)
            }
        }
        //let fb_fix_info = unsafe{ &mut *(data as *mut fb_fix_screeninfo) };
        //Ok(())
    }
    fn as_any_ref(&self) -> &dyn Any {
        self
    }
}

const FBIOGET_FSCREENINFO: u32 = 0x4602;
const FBIOGET_VSCREENINFO: u32 = 0x4600;

#[repr(C)]
pub struct fb_fix_screeninfo {
    pub id: [u8; 16],    /* identification string eg "TT Builtin" */
    pub smem_start: u64, /* Start of frame buffer mem */
    /* (physical address) */
    pub smem_len: u32,    /* Length of frame buffer mem */
    pub _type: u32,       /* see FB_TYPE_*		*/
    pub type_aux: u32,    /* Interleave for interleaved Planes */
    pub visual: u32,      /* see FB_VISUAL_*		*/
    pub xpanstep: u16,    /* zero if no hardware panning  */
    pub ypanstep: u16,    /* zero if no hardware panning  */
    pub ywrapstep: u16,   /* zero if no hardware ywrap    */
    pub line_length: u32, /* length of a line in bytes    */
    pub mmio_start: u64,  /* Start of Memory Mapped I/O   */
    /* (physical address) */
    pub mmio_len: u32, /* Length of Memory Mapped I/O  */
    pub accel: u32,    /* Indicate to driver which	*/
    /*  specific chip/card we have	*/
    pub capabilities: u16,  /* see FB_CAP_*			*/
    pub reserved: [u16; 2], /* Reserved for future compatibility */
}

#[repr(C)]
pub struct fb_var_screeninfo {
    pub xres: u32, /* visible resolution		*/
    pub yres: u32,
    pub xres_virtual: u32, /* virtual resolution		*/
    pub yres_virtual: u32,
    pub xoffset: u32, /* offset from virtual to visible */
    pub yoffset: u32, /* resolution			*/

    pub bits_per_pixel: u32, /* guess what			*/
    pub grayscale: u32,      /* 0 = color, 1 = grayscale,	*/
    /* >1 = FOURCC			*/
    pub red: fb_bitfield,   /* bitfield in fb mem if true color, */
    pub green: fb_bitfield, /* else only length is significant */
    pub blue: fb_bitfield,
    pub transp: fb_bitfield, /* transparency			*/

    pub nonstd: u32, /* != 0 Non standard pixel format */

    pub activate: u32, /* see FB_ACTIVATE_*		*/

    pub height: u32, /* height of picture in mm    */
    pub width: u32,  /* width of picture in mm     */

    pub accel_flags: u32, /* (OBSOLETE) see fb_info.flags */

    /* Timing: All values in pixclocks, except pixclock (of course) */
    pub pixclock: u32,     /* pixel clock in ps (pico seconds) */
    pub left_margin: u32,  /* time from sync to picture	*/
    pub right_margin: u32, /* time from picture to sync	*/
    pub upper_margin: u32, /* time from sync to picture	*/
    pub lower_margin: u32,
    pub hsync_len: u32,     /* length of horizontal sync	*/
    pub vsync_len: u32,     /* length of vertical sync	*/
    pub sync: u32,          /* see FB_SYNC_*		*/
    pub vmode: u32,         /* see FB_VMODE_*		*/
    pub rotate: u32,        /* angle we rotate counter clockwise */
    pub colorspace: u32,    /* colorspace for FOURCC-based modes */
    pub reserved: [u32; 4], /* Reserved for future compatibility */
}

#[repr(C)]
pub struct fb_bitfield {
    pub offset: u32, /* beginning of bitfield	*/
    pub length: u32, /* length of bitfield		*/
    pub msb_right: u32, /* != 0 : Most significant bit is */
                     /* right */
}
