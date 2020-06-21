use alloc::{collections::BTreeMap, string::String};
use core::slice;

use device_tree::{DeviceTree, Node};

use super::bus::virtio_mmio::virtio_probe;
use super::serial::uart16550;
use super::CMDLINE;
use crate::memory::phys_to_virt;
use spin::RwLock;

const DEVICE_TREE_MAGIC: u32 = 0xd00dfeed;

lazy_static! {
    pub static ref DEVICE_TREE_REGISTRY: RwLock<BTreeMap<&'static str, fn(&Node)>> =
        RwLock::new(BTreeMap::new());
}

fn walk_dt_node(dt: &Node) {
    if let Ok(compatible) = dt.prop_str("compatible") {
        let registry = DEVICE_TREE_REGISTRY.read();
        if let Some(f) = registry.get(compatible) {
            f(dt);
        }
    }
    if let Ok(bootargs) = dt.prop_str("bootargs") {
        if bootargs.len() > 0 {
            info!("Kernel cmdline: {}", bootargs);
            *CMDLINE.write() = String::from(bootargs);
        }
    }
    for child in dt.children.iter() {
        walk_dt_node(child);
    }
}

struct DtbHeader {
    magic: u32,
    size: u32,
}

pub fn init(dtb: usize) {
    let header = unsafe { &*(dtb as *const DtbHeader) };
    let magic = u32::from_be(header.magic);
    if magic == DEVICE_TREE_MAGIC {
        let size = u32::from_be(header.size);
        let dtb_data = unsafe { slice::from_raw_parts(dtb as *const u8, size as usize) };
        if let Ok(dt) = DeviceTree::load(dtb_data) {
            walk_dt_node(&dt.root);
        }
    }
}
