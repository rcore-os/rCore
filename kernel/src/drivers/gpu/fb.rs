//! Framebuffer

use crate::fs::vga::{fb_fix_screeninfo, fb_var_screeninfo};
use alloc::string::String;
use core::fmt;
use lazy_static::lazy_static;
use log::*;
use spin::Mutex;

/// Framebuffer information
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FramebufferInfo {
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
    pub depth: ColorDepth,
    /// color encoding format of RGBA
    pub format: ColorFormat,

    /// phsyical address
    pub paddr: usize,
    /// virtual address
    pub vaddr: usize,
    /// screen buffer size
    pub screen_size: usize,
}

pub type FramebufferResult = Result<FramebufferInfo, String>;

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum ColorDepth {
    ColorDepth8 = 8,
    ColorDepth16 = 16,
    ColorDepth24 = 24,
    ColorDepth32 = 32,
}
use self::ColorDepth::*;

impl ColorDepth {
    pub fn try_from(depth: u32) -> Result<Self, String> {
        match depth {
            8 => Ok(ColorDepth8),
            16 => Ok(ColorDepth16),
            32 => Ok(ColorDepth32),
            24 => Ok(ColorDepth24),
            _ => Err(format!("unsupported color depth {}", depth)),
        }
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum ColorFormat {
    RGB332,
    RGB565,
    RGBA8888, // QEMU and low version RPi use RGBA
    BGRA8888, // RPi3 B+ uses BGRA
    VgaPalette,
}

#[repr(C)]
union ColorBuffer {
    base_addr: usize,
    buf8: &'static mut [u8],
    buf16: &'static mut [u16],
    buf32: &'static mut [u32],
}

impl ColorBuffer {
    fn new(color_depth: ColorDepth, vaddr: usize, size: usize) -> ColorBuffer {
        unsafe {
            match color_depth {
                ColorDepth8 => ColorBuffer {
                    buf8: core::slice::from_raw_parts_mut(vaddr as *mut u8, size),
                },
                ColorDepth16 => ColorBuffer {
                    buf16: core::slice::from_raw_parts_mut(vaddr as *mut u16, size / 2),
                },
                ColorDepth24 => ColorBuffer {
                    buf8: core::slice::from_raw_parts_mut(vaddr as *mut u8, size),
                },
                ColorDepth32 => ColorBuffer {
                    buf32: core::slice::from_raw_parts_mut(vaddr as *mut u32, size / 4),
                },
            }
        }
    }

    #[inline]
    fn read8(&self, index: u32) -> u8 {
        unsafe { self.buf8[index as usize] }
    }

    #[inline]
    fn read16(&self, index: u32) -> u16 {
        unsafe { self.buf16[index as usize] }
    }

    #[inline]
    fn read32(&self, index: u32) -> u32 {
        unsafe { self.buf32[index as usize] }
    }

    #[inline]
    fn write8(&mut self, index: u32, pixel: u8) {
        unsafe { self.buf8[index as usize] = pixel }
    }

    #[inline]
    fn write16(&mut self, index: u32, pixel: u16) {
        unsafe { self.buf16[index as usize] = pixel }
    }

    #[inline]
    fn write24(&mut self, index: u32, pixel: u32) {
        let index = index * 3;
        unsafe { self.buf8[2 + index as usize] = (pixel >> 16) as u8 }
        unsafe { self.buf8[1 + index as usize] = (pixel >> 8) as u8 }
        unsafe { self.buf8[index as usize] = pixel as u8 }
    }

    #[inline]
    fn write32(&mut self, index: u32, pixel: u32) {
        unsafe { self.buf32[index as usize] = pixel }
    }
}

impl fmt::Debug for ColorBuffer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ColorBuffer({:#x})", unsafe { self.base_addr })
    }
}

/// Framebuffer structure
#[derive(Debug)]
pub struct Framebuffer {
    pub fb_info: FramebufferInfo,
    buf: ColorBuffer,
}

impl Framebuffer {
    fn new(info: FramebufferInfo) -> Framebuffer {
        Framebuffer {
            buf: ColorBuffer::new(info.depth, info.vaddr, info.screen_size),
            fb_info: info,
        }
    }

    #[inline]
    pub fn base_addr(&self) -> usize {
        unsafe { self.buf.base_addr }
    }

    #[inline]
    pub fn paddr(&self) -> usize {
        self.fb_info.paddr
    }

    #[inline]
    pub fn framebuffer_size(&self) -> usize {
        self.fb_info.screen_size
    }

    /// Read pixel at `(x, y)`.
    #[inline]
    pub fn read(&self, x: u32, y: u32) -> u32 {
        match self.fb_info.depth {
            ColorDepth8 => self.buf.read8(y * self.fb_info.xres + x) as u32,
            ColorDepth16 => self.buf.read16(y * self.fb_info.xres + x) as u32,
            ColorDepth24 => unimplemented!(),
            ColorDepth32 => self.buf.read32(y * self.fb_info.xres + x),
        }
    }

    /// Write pixel at `(x, y)`.
    #[inline]
    pub fn write(&mut self, x: u32, y: u32, pixel: u32) {
        match self.fb_info.depth {
            ColorDepth8 => self.buf.write8(y * self.fb_info.xres + x, pixel as u8),
            ColorDepth16 => self.buf.write16(y * self.fb_info.xres + x, pixel as u16),
            ColorDepth24 => self.buf.write24(y * self.fb_info.xres + x, pixel),
            ColorDepth32 => self.buf.write32(y * self.fb_info.xres + x, pixel),
        }
    }

    /// Copy buffer `[src_off .. src_off + size]` to `[dst_off .. dst_off + size]`.
    /// `dst_off`, `src_off` and `size` must be aligned with `usize`.
    pub fn copy(&mut self, dst_off: usize, src_off: usize, size: usize) {
        const USIZE: usize = core::mem::size_of::<usize>();
        let mut dst = self.base_addr() + dst_off;
        let mut src = self.base_addr() + src_off;
        let src_end = src + size;
        while src < src_end {
            unsafe { *(dst as *mut usize) = *(src as *mut usize) }
            dst += USIZE;
            src += USIZE;
        }
    }

    /// Fill buffer `[offset .. offset + size]` with `pixel`.
    /// `offset` and `size` must be aligned with `usize`.
    pub fn fill(&mut self, offset: usize, size: usize, pixel: u32) {
        const USIZE: usize = core::mem::size_of::<usize>();
        let mut value: usize = 0;
        let depth = self.fb_info.depth as usize;
        let repeat = USIZE * 8 / depth;
        let mask = ((1u64 << depth) - 1) as usize;
        for _i in 0..repeat {
            value <<= depth;
            value += pixel as usize & mask;
        }

        let mut start = self.base_addr() + offset;
        let end = start + size;
        while start < end {
            unsafe { *(start as *mut usize) = value }
            start += USIZE;
        }
    }

    /// Fill the entire buffer with `0`.
    pub fn clear(&mut self) {
        self.fill(0, self.fb_info.screen_size, 0);
    }

    pub fn fill_var_screeninfo(&self, var_info: &mut fb_var_screeninfo) {
        var_info.xres = self.fb_info.xres;
        var_info.yres = self.fb_info.yres;
        var_info.xres_virtual = self.fb_info.xres_virtual;
        var_info.yres_virtual = self.fb_info.yres_virtual;
        var_info.xoffset = self.fb_info.xoffset;
        var_info.yoffset = self.fb_info.yoffset;
        var_info.bits_per_pixel = self.fb_info.depth as u32;
    }

    pub fn fill_fix_screeninfo(&self, fix_info: &mut fb_fix_screeninfo) {
        fix_info.line_length = self.fb_info.xres * self.fb_info.depth as u32 / 8
    }
}

use rcore_console::embedded_graphics::prelude::*;
use rcore_console::Rgb888;

/// To be the backend of rCore `Console`
impl Drawing<Rgb888> for Framebuffer {
    fn draw<T>(&mut self, item: T)
    where
        T: IntoIterator<Item = Pixel<Rgb888>>,
    {
        for Pixel(point, color) in item {
            let pixel = color.pack32(self.fb_info.format);
            self.write(point[0] as u32, point[1] as u32, pixel);
        }
    }
}

trait ColorEncode {
    /// Encode `Rgb888` to a pixel in the framebuffer
    fn pack32(&self, format: ColorFormat) -> u32;
}

impl ColorEncode for Rgb888 {
    #[inline]
    fn pack32(&self, format: ColorFormat) -> u32 {
        match format {
            ColorFormat::RGB332 => {
                (((self.r() >> 5) << 5) | ((self.g() >> 5) << 2) | (self.b() >> 6)) as u32
            }
            ColorFormat::RGB565 => {
                (((self.r() as u16 & 0xF8) << 8)
                    | ((self.g() as u16 & 0xFC) << 3)
                    | (self.b() as u16 >> 3)) as u32
            }
            ColorFormat::RGBA8888 => {
                ((self.r() as u32) << 16) | ((self.g() as u32) << 8) | (self.b() as u32)
            }
            ColorFormat::BGRA8888 => {
                ((self.b() as u32) << 16) | ((self.g() as u32) << 8) | (self.r() as u32)
            }
            _ => unimplemented!(),
        }
    }
}

pub static FRAME_BUFFER: Mutex<Option<Framebuffer>> = Mutex::new(None);

/// Initialize framebuffer
///
/// Called in arch mod if the board have a framebuffer
pub fn init(info: FramebufferInfo) {
    let fb = Framebuffer::new(info);
    info!("framebuffer: init end\n{:#x?}", fb);
    *FRAME_BUFFER.lock() = Some(fb);
}
