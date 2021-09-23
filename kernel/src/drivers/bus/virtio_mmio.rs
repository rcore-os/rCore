use super::super::block::virtio_blk;
use super::super::gpu::virtio_gpu;
use super::super::input::virtio_input;
use super::super::net::virtio_net;
use super::super::serial::virtio_console;
use crate::drivers::device_tree::DEVICE_TREE_REGISTRY;
use crate::memory::phys_to_virt;
use device_tree::util::SliceRead;
use device_tree::Node;
use log::*;
use rcore_memory::PAGE_SIZE;
use virtio_drivers::{DeviceType, VirtIOHeader};

pub fn virtio_probe(node: &Node) {
    let reg = match node.prop_raw("reg") {
        Some(reg) => reg,
        _ => return,
    };
    let paddr = reg.as_slice().read_be_u64(0).unwrap();
    let vaddr = phys_to_virt(paddr as usize);
    let size = reg.as_slice().read_be_u64(8).unwrap();
    // assuming one page
    assert_eq!(size as usize, PAGE_SIZE);
    let header = unsafe { &mut *(vaddr as *mut VirtIOHeader) };
    if !header.verify() {
        // only support legacy device
        return;
    }
    info!(
        "Detected virtio device with vendor id: {:#X}",
        header.vendor_id()
    );
    info!("Device tree node {:?}", node);
    match header.device_type() {
        DeviceType::Network => virtio_net::init(header),
        DeviceType::Block => virtio_blk::init(header),
        DeviceType::GPU => virtio_gpu::init(header),
        DeviceType::Input => virtio_input::init(header),
        DeviceType::Console => virtio_console::init(node, header),
        t => warn!("Unrecognized virtio device: {:?}", t),
    }
}

pub fn driver_init() {
    DEVICE_TREE_REGISTRY
        .write()
        .insert("virtio,mmio", virtio_probe);
}
