#![allow(dead_code)]

use crate::drivers::Driver;
use alloc::sync::Arc;

const BLOCK_SIZE: usize = 512;

pub fn test_read(driver: Arc<dyn Driver>) {
    // print out the first section of the sd_card.
    let mut section: [u8; BLOCK_SIZE] = [0; BLOCK_SIZE];
    println!("Trying to fetch the first section of the SD card.");
    if !driver.read_block(0, &mut section) {
        error!("Failed in fetching.");
        return;
    }
    println!("Content:");
    for i in 0..32 {
        for j in 0..16 {
            print!("{:02X} ", section[i * 16 + j]);
        }
        println!("");
    }
    println!("");
    if section[510] != 0x55 || section[511] != 0xAA {
        println!("The first section is not an MBR section!");
        println!("Maybe you are working on qemu using raw image.");
        println!("Change the -sd argument to raspibian.img.");
        return;
    }
    let mut start_pos = 446; // start position of the partion table
    for entry in 0..4 {
        print!("Partion entry #{}: ", entry);
        let partion_type = section[start_pos + 0x4];
        fn partion_type_map(partion_type: u8) -> &'static str {
            match partion_type {
                0x00 => "Empty",
                0x0c => "FAT32",
                0x83 => "Linux",
                0x82 => "Swap",
                _ => "Not supported",
            }
        }
        print!("{:^14}", partion_type_map(partion_type));
        if partion_type != 0x00 {
            let start_section: u32 = (section[start_pos + 0x8] as u32)
                | (section[start_pos + 0x9] as u32) << 8
                | (section[start_pos + 0xa] as u32) << 16
                | (section[start_pos + 0xb] as u32) << 24;
            let total_section: u32 = (section[start_pos + 0xc] as u32)
                | (section[start_pos + 0xd] as u32) << 8
                | (section[start_pos + 0xe] as u32) << 16
                | (section[start_pos + 0xf] as u32) << 24;
            print!(
                " start section no. = {}, a total of {} sections in use.",
                start_section, total_section
            );
        }
        println!("");
        start_pos += 16;
    }
}

pub fn test_write(driver: Arc<dyn Driver>) {
    let mut section: [u8; BLOCK_SIZE] = [0; BLOCK_SIZE];
    let mut deadbeef: [u8; BLOCK_SIZE] = [0; BLOCK_SIZE];
    println!("Trying to fetch the second section of the SD card.");
    if !driver.read_block(1, &mut section) {
        error!("Failed in fetching.");
        return;
    }
    println!("Content:");
    for i in 0..32 {
        for j in 0..16 {
            print!("{:02X} ", section[i * 16 + j]);
        }
        println!("");
    }
    println!("");

    for i in 0..512 / 4 {
        deadbeef[i * 4 + 0] = 0xDE;
        deadbeef[i * 4 + 1] = 0xAD;
        deadbeef[i * 4 + 2] = 0xBE;
        deadbeef[i * 4 + 3] = 0xEF;
    }

    if !driver.write_block(1, &deadbeef) {
        error!("Failed in writing.");
        return;
    }
    if !driver.read_block(1, &mut deadbeef) {
        error!("Failed in checking.");
        return;
    }
    println!("Re-fetched content:");
    for i in 0..32 {
        for j in 0..16 {
            print!("{:02X} ", deadbeef[i * 16 + j]);
        }
        println!("");
    }
    println!("");
    if !driver.write_block(1, &section) {
        error!("Failed in writing back.");
        return;
    }
    for i in 0..512 / 4 {
        if deadbeef[i * 4 + 0] != 0xDE
            || deadbeef[i * 4 + 1] != 0xAD
            || deadbeef[i * 4 + 2] != 0xBE
            || deadbeef[i * 4 + 3] != 0xEF
        {
            error!("Re-fetched content is wrong!");
            return;
        }
    }
    println!("Passed write check.");
}
