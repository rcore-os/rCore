#![allow(dead_code)]

use alloc::sync::Arc;
use rcore_fs::dev::Device;

const BLOCK_SIZE: usize = 4096;

#[cfg(target_arch = "aarch64")]
pub fn test_speed(device: &impl Device) {
    let mut section: [u8; BLOCK_SIZE] = [0; BLOCK_SIZE];
    let count = 4 * 1024;
    let mib = (count * BLOCK_SIZE) as f64 / 1024.0 / 1024.0;

    println!("================ Test read speed ================");
    println!("Reading {} blocks ({} MiB) ...", count, mib);
    let begin = crate::arch::timer::get_cycle();
    let read_block_id = 2333;
    for _ in 0..count {
        device.read_at(read_block_id, &mut section).unwrap();
    }
    let end = crate::arch::timer::get_cycle();
    let second = (end - begin) as f64 / 1000000.0;
    println!("Time used: {:.3} s", second);
    println!("Speed : {:.3} MiB/s", mib / second);

    println!("================ Test write speed ================");
    println!("Writing {} blocks ({} MiB) ...", count, mib);
    let write_block_id = 6666;
    device.read_at(write_block_id, &mut section).unwrap();
    let begin = crate::arch::timer::get_cycle();
    for _ in 0..count {
        device.write_at(write_block_id, &mut section).unwrap();
    }
    let end = crate::arch::timer::get_cycle();
    let second = (end - begin) as f64 / 1000000.0;
    println!("Time used: {:.3} s", second);
    println!("Speed : {:.3} MiB/s", mib / second);
}
