#[path = "../../../drivers/gpu/fb.rs"]
pub mod fb;

use crate::consts::KERNEL_OFFSET;
use crate::memory::phys_to_virt;
use fb::{ColorConfig, FramebufferInfo, FramebufferResult, FRAME_BUFFER};

pub fn init_driver() {
    #[cfg(not(feature = "nographic"))]
    fb::init();
}

pub fn probe_fb_info(width: u32, height: u32, depth: u32) -> FramebufferResult {
    let fb_info = FramebufferInfo {
        xres: 1024,
        yres: 768,
        xres_virtual: 1024,
        yres_virtual: 768,
        xoffset: 0,
        yoffset: 0,
        depth: 32,
        pitch: 1024, // TOKNOW
        bus_addr: 0xf100_0000,
        screen_size: 1024 * 768 * 4,
    };
    Ok((
        fb_info,
        fb::ColorConfig::BGRA8888,
        phys_to_virt(0xfd00_0000),
    ))
}
