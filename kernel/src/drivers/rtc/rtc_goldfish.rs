//! RTC driver for google,goldfish-rtc in QEMU
use super::super::RTC_DRIVERS;
use super::RtcDriver;
use crate::drivers::device_tree::DEVICE_TREE_REGISTRY;
use crate::util::read;
use crate::{
    arch::interrupt,
    drivers::{DeviceType, Driver},
};
use alloc::string::String;
use alloc::sync::Arc;
use device_tree::Node;

const TIMER_TIME_LOW: usize = 0x00;
const TIMER_TIME_HIGH: usize = 0x04;

pub struct RtcGoldfish {
    base: usize,
}

impl Driver for RtcGoldfish {
    fn try_handle_interrupt(&self, irq: Option<usize>) -> bool {
        false
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Rtc
    }

    fn get_id(&self) -> String {
        String::from("rtc_goldfish")
    }
}

impl RtcDriver for RtcGoldfish {
    // read seconds since 1970-01-01
    fn read_epoch(&self) -> u64 {
        let low: u32 = read(self.base + TIMER_TIME_LOW);
        let high: u32 = read(self.base + TIMER_TIME_HIGH);
        let ns = ((high as u64) << 32) | (low as u64);
        ns / 1_000_000_000u64
    }
}

fn init_dt(dt: &Node) {
    use crate::memory::phys_to_virt;
    let addr = dt.prop_u64("reg").unwrap() as usize;
    RTC_DRIVERS.write().push(Arc::new(RtcGoldfish {
        base: phys_to_virt(addr),
    }));
}

pub fn driver_init() {
    DEVICE_TREE_REGISTRY
        .write()
        .insert("google,goldfish-rtc", init_dt);
}
