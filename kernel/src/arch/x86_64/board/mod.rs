use crate::consts::KERNEL_OFFSET;
use crate::drivers::gpu::fb::{self, ColorDepth, ColorFormat, FramebufferInfo};
use crate::memory::phys_to_virt;
use core::mem::zeroed;
use rboot::BootInfo;

pub fn init_driver(boot_info: &BootInfo) {
    let info = &boot_info.graphic_info;
    let width = info.mode.resolution().0 as u32;
    let height = info.mode.resolution().1 as u32;

    let fb_info = FramebufferInfo {
        xres: width,
        yres: height,
        xres_virtual: width,
        yres_virtual: height,
        xoffset: 0,
        yoffset: 0,
        depth: ColorDepth::ColorDepth32,
        format: fb::ColorFormat::RGBA8888,
        paddr: info.fb_addr as usize,
        vaddr: phys_to_virt(info.fb_addr as usize),
        screen_size: info.fb_size as usize,
    };
    fb::init(fb_info);
}
