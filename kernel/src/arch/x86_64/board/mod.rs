#[path = "../../../drivers/gpu/fb.rs"]
pub mod fb;

use crate::consts::KERNEL_OFFSET;
use crate::memory::phys_to_virt;
use bootloader::bootinfo::{BootInfo, VbeModeInfo};
use core::mem::zeroed;
use fb::{ColorConfig, FramebufferInfo, FramebufferResult, FRAME_BUFFER};

static mut VBE_MODE: VbeModeInfo = VbeModeInfo {
    attributes: 0,
    win_a: 0,
    win_b: 0,
    granularity: 0,
    window_size: 0,
    segment_a: 0,
    segment_b: 0,
    _1: 0,
    pitch: 0,
    width: 0,
    height: 0,
    w_char: 0,
    y_char: 0,
    planes: 0,
    bpp: 0,
    banks: 0,
    memory_model: 0,
    bank_size: 0,
    image_pages: 0,
    _2: 0,
    red_mask: 0,
    red_position: 0,
    green_mask: 0,
    green_position: 0,
    blue_mask: 0,
    blue_position: 0,
    rsv_mask: 0,
    rsv_position: 0,
    directcolor_attributes: 0,
    framebuffer: 0,
};

pub fn init_driver(boot_info: &BootInfo) {
    unsafe {
        VBE_MODE = boot_info.vbe_info;
    }
    #[cfg(not(feature = "nographic"))]
    fb::init();
}

pub fn probe_fb_info(_width: u32, _height: u32, _depth: u32) -> FramebufferResult {
    let width = unsafe { VBE_MODE.width as u32 };
    let height = unsafe { VBE_MODE.height as u32 };
    let pitch = unsafe { VBE_MODE.pitch as u32 };
    let framebuffer = unsafe { VBE_MODE.framebuffer as usize };
    let depth = unsafe { VBE_MODE.bpp as u32 };
    let fb_info = FramebufferInfo {
        xres: width,
        yres: height,
        xres_virtual: width,
        yres_virtual: height,
        xoffset: 0,
        yoffset: 0,
        depth: depth,
        pitch: pitch, // TOKNOW
        bus_addr: framebuffer as u32,
        screen_size: width * height * (depth / 8),
    };
    let config = if depth == 8 {
        fb::ColorConfig::RGB332
    } else if depth == 16 {
        fb::ColorConfig::RGB565
    } else {
        // assume BGRA8888 for now
        fb::ColorConfig::RGBA8888
    };
    Ok((fb_info, config, phys_to_virt(framebuffer)))
}
