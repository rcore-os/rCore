use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;
use core::fmt;
use core::mem::size_of;
use core::mem::transmute_copy;
use core::slice;

use bitflags::*;
use device_tree::util::SliceRead;
use device_tree::Node;
use log::*;
use rcore_memory::PAGE_SIZE;
use volatile::Volatile;

use crate::arch::cpu;
use crate::sync::SpinNoIrqLock as Mutex;

use super::super::bus::virtio_mmio::*;
use super::super::{DeviceType, Driver, DRIVERS, IRQ_MANAGER};
use crate::memory::phys_to_virt;

struct VirtIOInput {
    interrupt_parent: u32,
    interrupt: u32,
    header: &'static mut VirtIOHeader,
    // 0 for event, 1 for status
    queues: [VirtIOVirtqueue; 2],
    x: isize,
    y: isize,
}

const VIRTIO_INPUT_CFG_UNSET: u8 = 0x00;
const VIRTIO_INPUT_CFG_ID_NAME: u8 = 0x01;
const VIRTIO_INPUT_CFG_ID_SERIAL: u8 = 0x02;
const VIRTIO_INPUT_CFG_ID_DEVIDS: u8 = 0x03;
const VIRTIO_INPUT_CFG_PROP_BITS: u8 = 0x10;
const VIRTIO_INPUT_CFG_EV_BITS: u8 = 0x11;
const VIRTIO_INPUT_CFG_ABS_INFO: u8 = 0x12;

#[repr(C)]
#[derive(Debug)]
struct VirtIOInputConfig {
    select: Volatile<u8>,
    subsel: Volatile<u8>,
    size: u8,
    reversed: [u8; 5],
    data: [u8; 32],
}

#[repr(C)]
#[derive(Debug)]
struct VirtIOInputAbsInfo {
    min: u32,
    max: u32,
    fuzz: u32,
    flat: u32,
    res: u32,
}

#[repr(C)]
#[derive(Debug)]
struct VirtIOInputDevIDs {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

#[repr(C)]
#[derive(Clone, Default)]
struct VirtIOInputEvent {
    event_type: u16,
    code: u16,
    value: u32,
}

impl fmt::Display for VirtIOInputEvent {
    // linux event codes
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.event_type {
            0 => match self.code {
                0 => write!(f, "SYN_REPORT"),
                _ => write!(f, "Unknown SYN code {}", self.code),
            },
            2 => match self.code {
                0 => write!(f, "REL_X {}", self.value),
                1 => write!(f, "REL_Y {}", self.value),
                _ => write!(f, "Unknown REL code {}", self.code),
            },
            _ => write!(f, "Unknown event type {}", self.event_type),
        }
    }
}

bitflags! {
    struct VirtIOInputFeature : u64 {
        // device independent
        const NOTIFY_ON_EMPTY = 1 << 24; // legacy
        const ANY_LAYOUT = 1 << 27; // legacy
        const RING_INDIRECT_DESC = 1 << 28;
        const RING_EVENT_IDX = 1 << 29;
        const UNUSED = 1 << 30; // legacy
        const VERSION_1 = 1 << 32; // detect legacy
        const ACCESS_PLATFORM = 1 << 33; // since virtio v1.1
        const RING_PACKED = 1 << 34; // since virtio v1.1
        const IN_ORDER = 1 << 35; // since virtio v1.1
        const ORDER_PLATFORM = 1 << 36; // since virtio v1.1
        const SR_IOV = 1 << 37; // since virtio v1.1
        const NOTIFICATION_DATA = 1 << 38; // since virtio v1.1
    }
}

const VIRTIO_QUEUE_EVENT: usize = 0;
const VIRTIO_QUEUE_STATUS: usize = 1;

pub struct VirtIOInputDriver(Mutex<VirtIOInput>);

impl VirtIOInput {
    fn try_handle_interrupt(&mut self, _irq: Option<u32>) -> bool {
        // for simplicity
        if cpu::id() > 0 {
            return false;
        }

        let interrupt = self.header.interrupt_status.read();
        if interrupt != 0 {
            self.header.interrupt_ack.write(interrupt);
            debug!("Got interrupt {:?}", interrupt);
            loop {
                if let Some((input, output, _, _)) = self.queues[VIRTIO_QUEUE_EVENT].get() {
                    let event: VirtIOInputEvent = unsafe { transmute_copy(&input[0][0]) };
                    if event.event_type == 2 && event.code == 0 {
                        // X
                        self.x += event.value as isize;
                    } else if event.event_type == 2 && event.code == 1 {
                        // X
                        self.y += event.value as isize;
                    }
                    trace!("got {}", event);
                    self.queues[VIRTIO_QUEUE_EVENT].add(&input, &output, 0);
                } else {
                    break;
                }
            }
            println!("mouse is at x {} y {}", self.x, self.y);
            return true;
        }
        return false;
    }
}

impl Driver for VirtIOInputDriver {
    fn try_handle_interrupt(&self, irq: Option<u32>) -> bool {
        self.0.lock().try_handle_interrupt(irq)
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Input
    }

    fn get_id(&self) -> String {
        String::from("virtio_input")
    }
}

pub fn virtio_input_init(node: &Node) {
    let reg = node.prop_raw("reg").unwrap();
    let paddr = reg.as_slice().read_be_u64(0).unwrap();
    let vaddr = phys_to_virt(paddr as usize);
    let header = unsafe { &mut *(vaddr as *mut VirtIOHeader) };

    header.status.write(VirtIODeviceStatus::DRIVER.bits());

    let device_features_bits = header.read_device_features();
    let device_features = VirtIOInputFeature::from_bits_truncate(device_features_bits);
    println!("Device features {:?}", device_features);

    // negotiate these flags only
    let supported_features = VirtIOInputFeature::empty();
    let driver_features = (device_features & supported_features).bits();
    header.write_driver_features(driver_features);

    // read configuration space
    let config = unsafe { &mut *((vaddr + VIRTIO_CONFIG_SPACE_OFFSET) as *mut VirtIOInputConfig) };
    info!("Config: {:?}", config);

    // virtio 4.2.4 Legacy interface
    // configure two virtqueues: ingress and egress
    header.guest_page_size.write(PAGE_SIZE as u32); // one page

    let queue_num = 32;
    let queues = [
        VirtIOVirtqueue::new(header, VIRTIO_QUEUE_EVENT, queue_num),
        VirtIOVirtqueue::new(header, VIRTIO_QUEUE_STATUS, queue_num),
    ];
    let mut driver = VirtIOInput {
        interrupt: node.prop_u32("interrupts").unwrap(),
        interrupt_parent: node.prop_u32("interrupt-parent").unwrap(),
        header,
        queues,
        x: 0,
        y: 0,
    };

    let buffer = vec![VirtIOInputEvent::default(); queue_num];
    let input_buffers: &mut [VirtIOInputEvent] = Box::leak(buffer.into_boxed_slice());
    for i in 0..queue_num {
        let buffer = unsafe {
            slice::from_raw_parts(
                (&input_buffers[i]) as *const VirtIOInputEvent as *const u8,
                size_of::<VirtIOInputEvent>(),
            )
        };
        driver.queues[VIRTIO_QUEUE_EVENT].add(&[buffer], &[], 0);
    }

    driver
        .header
        .status
        .write(VirtIODeviceStatus::DRIVER_OK.bits());

    let driver = Arc::new(VirtIOInputDriver(Mutex::new(driver)));
    IRQ_MANAGER.write().register_all(driver.clone());
    DRIVERS.write().push(driver);
}
