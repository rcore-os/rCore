use alloc::string::String;
use alloc::sync::Arc;

use virtio_drivers::{VirtIOGpu, VirtIOHeader};

use super::super::{DeviceType, Driver, DRIVERS, IRQ_MANAGER};
use crate::memory::virt_to_phys;
use crate::{
    drivers::{BlockDriver, NetDriver},
    sync::SpinNoIrqLock as Mutex,
};

struct VirtIOGpuDriver(Mutex<VirtIOGpu<'static>>);

impl Driver for VirtIOGpuDriver {
    fn try_handle_interrupt(&self, _irq: Option<u32>) -> bool {
        self.0.lock().ack_interrupt()
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Gpu
    }

    fn get_id(&self) -> String {
        format!("virtio_gpu")
    }

    fn as_block(&self) -> Option<&dyn BlockDriver> {
        None
    }

    fn as_net(&self) -> Option<&dyn NetDriver> {
        None
    }
}

pub fn init(header: &'static mut VirtIOHeader) {
    let mut gpu = VirtIOGpu::new(header).expect("failed to create gpu driver");
    let framebuffer = gpu.setup_framebuffer().expect("failed to get fb");
    let fb_vaddr = framebuffer.as_ptr() as usize;
    let fb_size = framebuffer.len();
    let (width, height) = gpu.resolution();

    // test
    // super::test::mandelbrot(width, height, fb_vaddr as _);
    // gpu.flush().expect("failed to flush");

    use super::fb;
    fb::init(fb::FramebufferInfo {
        xres: width,
        yres: height,
        xres_virtual: width,
        yres_virtual: height,
        xoffset: 0,
        yoffset: 0,
        depth: fb::ColorDepth::ColorDepth32,
        format: fb::ColorFormat::RGBA8888,
        paddr: virt_to_phys(fb_vaddr),
        vaddr: fb_vaddr,
        screen_size: fb_size,
    });

    let driver = Arc::new(VirtIOGpuDriver(Mutex::new(gpu)));
    IRQ_MANAGER.write().register_all(driver.clone());
    DRIVERS.write().push(driver);
}
