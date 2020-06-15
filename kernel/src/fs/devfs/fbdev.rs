//! Implement INode for framebuffer

use crate::drivers::gpu::fb::{ColorFormat, FramebufferInfo, FRAME_BUFFER};
use crate::syscall::MmapProt;
use core::any::Any;

use rcore_fs::vfs::*;
use rcore_memory::memory_set::handler::Linear;

#[derive(Default)]
pub struct Fbdev;

impl INode for Fbdev {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        info!(
            "fbdev read_at: offset={:#x} buf_len={:#x}",
            offset,
            buf.len()
        );
        if let Some(fb) = FRAME_BUFFER.read().as_ref() {
            Ok(fb.read_at(offset, buf))
        } else {
            Err(FsError::NoDevice)
        }
    }
    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        info!(
            "fbdev write_at: offset={:#x} buf_len={:#x}",
            offset,
            buf.len()
        );
        if let Some(fb) = FRAME_BUFFER.write().as_mut() {
            let count = fb.write_at(offset, buf);
            if count == buf.len() {
                Ok(count)
            } else {
                Err(FsError::NoDeviceSpace)
            }
        } else {
            Err(FsError::NoDevice)
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
            size: 0,
            blk_size: 0,
            blocks: 0,
            atime: Timespec { sec: 0, nsec: 0 },
            mtime: Timespec { sec: 0, nsec: 0 },
            ctime: Timespec { sec: 0, nsec: 0 },
            type_: FileType::CharDevice,
            mode: 0o660,
            nlinks: 1,
            uid: 0,
            gid: 0,
            rdev: make_rdev(29, 0),
        })
    }
    fn io_control(&self, cmd: u32, data: usize) -> Result<usize> {
        const FBIOGET_VSCREENINFO: u32 = 0x4600;
        const FBIOGET_FSCREENINFO: u32 = 0x4602;

        match cmd {
            FBIOGET_FSCREENINFO => {
                let fb_fix_info = unsafe { &mut *(data as *mut FbFixScreeninfo) };
                if let Some(fb) = FRAME_BUFFER.read().as_ref() {
                    fb_fix_info.fill_from(&fb.fb_info);
                }
                Ok(0)
            }
            FBIOGET_VSCREENINFO => {
                let fb_var_info = unsafe { &mut *(data as *mut FbVarScreeninfo) };
                if let Some(fb) = FRAME_BUFFER.read().as_ref() {
                    fb_var_info.fill_from(&fb.fb_info);
                }
                Ok(0)
            }
            _ => {
                warn!("use never support ioctl !");
                Err(FsError::NotSupported)
            }
        }
    }
    fn mmap(&self, area: MMapArea) -> Result<()> {
        let attr = MmapProt::from_bits_truncate(area.prot).to_attr();
        #[cfg(target_arch = "aarch64")]
        let attr = attr.mmio(crate::arch::paging::MMIOType::NormalNonCacheable as u8);

        if let Some(fb) = FRAME_BUFFER.read().as_ref() {
            if area.offset + area.end_vaddr - area.start_vaddr > fb.framebuffer_size() {
                return Err(FsError::NoDeviceSpace);
            }
            let thread = unsafe { crate::process::current_thread() };
            thread.vm.lock().push(
                area.start_vaddr,
                area.end_vaddr,
                attr,
                Linear::new((fb.paddr() + area.offset - area.start_vaddr) as isize),
                "mmap_file",
            );
            Ok(())
        } else {
            Err(FsError::NoDevice)
        }
    }
    fn as_any_ref(&self) -> &dyn Any {
        self
    }
}

#[repr(u32)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
enum FbType {
    /// Packed Pixels
    PackedPixels = 0,
    /// Non interleaved planes
    Planes = 1,
    /// Interleaved planes
    InterleavedPlanes = 2,
    /// Text/attributes
    Text = 3,
    /// EGA/VGA planes
    VgaPlanes = 4,
    /// Type identified by a V4L2 FOURCC
    FourCC = 5,
}

#[repr(u32)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
enum FbVisual {
    /// Monochr. 1=Black 0=White
    Mono01 = 0,
    /// Monochr. 1=White 0=Black
    Mono10 = 1,
    /// True color
    TrueColor = 2,
    /// Pseudo color (like atari)
    PseudoColor = 3,
    /// Direct color
    DirectColor = 4,
    /// Pseudo color readonly
    StaticPseudoColor = 5,
    /// Visual identified by a V4L2 FOURCC
    FourCC = 6,
}

/// No hardware accelerator
const FB_ACCEL_NONE: u32 = 0;

#[repr(C)]
#[derive(Debug)]
struct FbFixScreeninfo {
    /// identification string eg "TT Builtin"
    id: [u8; 16],
    /// Start of frame buffer mem (physical address)
    smem_start: u64,
    /// Length of frame buffer mem
    smem_len: u32,
    /// see FB_TYPE_*
    type_: FbType,
    /// Interleave for interleaved Planes
    type_aux: u32,
    /// see FB_VISUAL_*
    visual: FbVisual,
    /// zero if no hardware panning
    xpanstep: u16,
    /// zero if no hardware panning
    ypanstep: u16,
    /// zero if no hardware ywrap
    ywrapstep: u16,
    /// length of a line in bytes
    line_length: u32,
    /// Start of Memory Mapped I/O (physical address)
    mmio_start: u64,
    /// Length of Memory Mapped I/O
    mmio_len: u32,
    /// Indicate to driver which specific chip/card we have
    accel: u32,
    /// see FB_CAP_*
    capabilities: u16,
    /// Reserved for future compatibility
    reserved: [u16; 2],
}

#[repr(C)]
#[derive(Debug)]
struct FbVarScreeninfo {
    /// visible resolution x
    xres: u32,
    /// visible resolution y
    yres: u32,
    /// virtual resolution x
    xres_virtual: u32,
    /// virtual resolution y
    yres_virtual: u32,
    /// offset from virtual to visible x
    xoffset: u32,
    /// offset from virtual to visible y
    yoffset: u32,

    /// guess what
    bits_per_pixel: u32,
    /// 0 = color, 1 = grayscale, >1 = FOURCC
    grayscale: u32,
    /// bitfield in fb mem if true color, else only length is significant
    red: FbBitfield,
    green: FbBitfield,
    blue: FbBitfield,
    transp: FbBitfield,

    /// != 0 Non standard pixel format
    nonstd: u32,

    /// see FB_ACTIVATE_*
    activate: u32,

    /// height of picture in mm
    height: u32,
    /// width of picture in mm
    width: u32,
    /// (OBSOLETE) see fb_info.flags
    accel_flags: u32,

    /* Timing: All values in pixclocks, except pixclock (of course) */
    /// pixel clock in ps (pico seconds)
    pixclock: u32,
    /// time from sync to picture
    left_margin: u32,
    /// time from picture to sync
    right_margin: u32,
    /// time from sync to picture
    upper_margin: u32,
    lower_margin: u32,
    /// length of horizontal sync
    hsync_len: u32,
    /// length of vertical sync
    vsync_len: u32,
    /// see FB_SYNC_*
    sync: u32,
    /// see FB_VMODE_*
    vmode: u32,
    /// angle we rotate counter clockwise
    rotate: u32,
    /// colorspace for FOURCC-based modes
    colorspace: u32,
    /// Reserved for future compatibility
    reserved: [u32; 4],
}

#[repr(C)]
#[derive(Debug)]
struct FbBitfield {
    /// beginning of bitfield
    offset: u32,
    /// length of bitfield
    length: u32,
    /// != 0 : Most significant bit is right
    msb_right: u32,
}

impl FbVarScreeninfo {
    fn fill_from(&mut self, fb_info: &FramebufferInfo) {
        self.xres = fb_info.xres;
        self.yres = fb_info.yres;
        self.xres_virtual = fb_info.xres_virtual;
        self.yres_virtual = fb_info.yres_virtual;
        self.xoffset = fb_info.xoffset;
        self.yoffset = fb_info.yoffset;
        self.bits_per_pixel = fb_info.depth as u32;
        let (rl, gl, bl, al, ro, go, bo, ao) = match fb_info.format {
            ColorFormat::RGB332 => (3, 3, 2, 0, 5, 3, 0, 0),
            ColorFormat::RGB565 => (5, 6, 5, 0, 11, 5, 0, 0),
            ColorFormat::RGBA8888 => (8, 8, 8, 8, 16, 8, 0, 24),
            ColorFormat::BGRA8888 => (8, 8, 8, 8, 0, 8, 16, 24),
            ColorFormat::VgaPalette => unimplemented!(),
        };
        self.blue = FbBitfield {
            offset: bo,
            length: bl,
            msb_right: 1,
        };
        self.green = FbBitfield {
            offset: go,
            length: gl,
            msb_right: 1,
        };
        self.red = FbBitfield {
            offset: ro,
            length: rl,
            msb_right: 1,
        };
        self.transp = FbBitfield {
            offset: ao,
            length: al,
            msb_right: 1,
        };
    }
}

impl FbFixScreeninfo {
    fn fill_from(&mut self, fb_info: &FramebufferInfo) {
        self.smem_start = fb_info.paddr as u64;
        self.smem_len = fb_info.screen_size as u32;

        self.type_ = FbType::PackedPixels;
        // self.type_aux = fb_info.type_aux;
        self.visual = FbVisual::TrueColor;

        // self.xpanstep = 0;
        // self.ypanstep = 0;
        // self.ywrapstep = 0;

        self.line_length = fb_info.xres * fb_info.depth as u32 / 8;

        self.mmio_start = 0;
        self.mmio_len = 0;
        self.accel = FB_ACCEL_NONE;
    }
}
