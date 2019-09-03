#[path = "../../../drivers/gpu/fb.rs"]
pub mod fb;

use crate::consts::KERNEL_OFFSET;
use crate::memory::phys_to_virt;
use core::mem::zeroed;
use fb::{ColorDepth, ColorFormat, FramebufferInfo, FramebufferResult, FRAME_BUFFER};
use rboot::{BootInfo, GraphicInfo};

static mut GRAPHIC_INFO: Option<GraphicInfo> = None;

pub fn init_driver(boot_info: &BootInfo) {
    unsafe {
        GRAPHIC_INFO = Some(boot_info.graphic_info);
    }
    #[cfg(not(feature = "nographic"))]
    fb::init();
}

pub fn probe_fb_info(_width: u32, _height: u32, _depth: u32) -> FramebufferResult {
    let info = unsafe { GRAPHIC_INFO.as_ref().unwrap() };
    let width = info.mode.resolution().0 as u32;
    let height = info.mode.resolution().1 as u32;
    let format = fb::ColorFormat::RGBA8888;
    Ok(FramebufferInfo {
        xres: width,
        yres: height,
        xres_virtual: width,
        yres_virtual: height,
        xoffset: 0,
        yoffset: 0,
        depth: ColorDepth::ColorDepth32,
        format: format,
        paddr: info.fb_addr as usize,
        vaddr: phys_to_virt(info.fb_addr as usize),
        screen_size: info.fb_size as usize,
    })
}
