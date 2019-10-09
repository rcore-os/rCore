#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(non_snake_case)]
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(non_upper_case_globals)]

use super::mailbox;
use crate::thread;
use bcm2837::emmc::*;
use core::mem;
use core::slice;
use core::time::Duration;

pub const BLOCK_SIZE: usize = 512;

const SD_CMD_TYPE_NORMAL: u32 = 0x0;
const SD_CMD_TYPE_SUSPEND: u32 = (1 << 22);
const SD_CMD_TYPE_RESUME: u32 = (2 << 22);
const SD_CMD_TYPE_ABORT: u32 = (3 << 22);
const SD_CMD_TYPE_MASK: u32 = (3 << 22);
const SD_CMD_ISDATA: u32 = (1 << 21);
const SD_CMD_IXCHK_EN: u32 = (1 << 20);
const SD_CMD_CRCCHK_EN: u32 = (1 << 19);
const SD_CMD_RSPNS_TYPE_NONE: u32 = 0; // For no response
const SD_CMD_RSPNS_TYPE_136: u32 = (1 << 16); // For response R2 (with CRC), R3,4 (no CRC)
const SD_CMD_RSPNS_TYPE_48: u32 = (2 << 16); // For responses R1, R5, R6, R7 (with CRC)
const SD_CMD_RSPNS_TYPE_48B: u32 = (3 << 16); // For responses R1b, R5b (with CRC)
const SD_CMD_RSPNS_TYPE_MASK: u32 = (3 << 16);
const SD_CMD_MULTI_BLOCK: u32 = (1 << 5);
const SD_CMD_DAT_DIR_HC: u32 = 0;
const SD_CMD_DAT_DIR_CH: u32 = (1 << 4);
const SD_CMD_AUTO_CMD_EN_NONE: u32 = 0;
const SD_CMD_AUTO_CMD_EN_CMD12: u32 = (1 << 2);
const SD_CMD_AUTO_CMD_EN_CMD23: u32 = (2 << 2);
const SD_CMD_BLKCNT_EN: u32 = (1 << 1);
const SD_CMD_DMA: u32 = 1;

const SD_ERR_BASE: u32 = 16;
const SD_ERR_CMD_TIMEOUT: u32 = 0;
const SD_ERR_CMD_CRC: u32 = 1;
const SD_ERR_CMD_END_BIT: u32 = 2;
const SD_ERR_CMD_INDEX: u32 = 3;
const SD_ERR_DATA_TIMEOUT: u32 = 4;
const SD_ERR_DATA_CRC: u32 = 5;
const SD_ERR_DATA_END_BIT: u32 = 6;
const SD_ERR_CURRENT_LIMIT: u32 = 7; // !(not supported)
const SD_ERR_AUTO_CMD12: u32 = 8;
const SD_ERR_ADMA: u32 = 9; // !(not supported)
const SD_ERR_TUNING: u32 = 10; // !(not supported)
const SD_ERR_RSVD: u32 = 11; // !(not supported)

const SD_ERR_MASK_CMD_TIMEOUT: u32 = (1 << (16 + SD_ERR_CMD_TIMEOUT));
const SD_ERR_MASK_CMD_CRC: u32 = (1 << (16 + SD_ERR_CMD_CRC));
const SD_ERR_MASK_CMD_END_BIT: u32 = (1 << (16 + SD_ERR_CMD_END_BIT));
const SD_ERR_MASK_CMD_INDEX: u32 = (1 << (16 + SD_ERR_CMD_INDEX));
const SD_ERR_MASK_DATA_TIMEOUT: u32 = (1 << (16 + SD_ERR_CMD_TIMEOUT));
const SD_ERR_MASK_DATA_CRC: u32 = (1 << (16 + SD_ERR_CMD_CRC));
const SD_ERR_MASK_DATA_END_BIT: u32 = (1 << (16 + SD_ERR_CMD_END_BIT));
// const SD_ERR_MASK_CURRENT_LIMIT: u32 = (1 << (16 + SD_ERR_CMD_CURRENT_LIMIT));
// const SD_ERR_MASK_AUTO_CMD12: u32 = (1 << (16 + SD_ERR_CMD_AUTO_CMD12));
// const SD_ERR_MASK_ADMA: u32 = (1 << (16 + SD_ERR_CMD_ADMA));
// const SD_ERR_MASK_TUNING: u32 = (1 << (16 + SD_ERR_CMD_TUNING));

const SD_COMMAND_COMPLETE: u32 = 1;
const SD_TRANSFER_COMPLETE: u32 = 1 << 1;
const SD_BLOCK_GAP_EVENT: u32 = 1 << 2;
const SD_DMA_INTERRUPT: u32 = 1 << 3;
const SD_BUFFER_WRITE_READY: u32 = 1 << 4;
const SD_BUFFER_READ_READY: u32 = 1 << 5;
const SD_CARD_INSERTION: u32 = 1 << 6;
const SD_CARD_REMOVAL: u32 = 1 << 7;
const SD_CARD_INTERRUPT: u32 = 1 << 8;

const SD_RESP_NONE: u32 = SD_CMD_RSPNS_TYPE_NONE;
const SD_RESP_R1: u32 = (SD_CMD_RSPNS_TYPE_48 | SD_CMD_CRCCHK_EN);
const SD_RESP_R1b: u32 = (SD_CMD_RSPNS_TYPE_48B | SD_CMD_CRCCHK_EN);
const SD_RESP_R2: u32 = (SD_CMD_RSPNS_TYPE_136 | SD_CMD_CRCCHK_EN);
const SD_RESP_R3: u32 = SD_CMD_RSPNS_TYPE_48;
const SD_RESP_R4: u32 = SD_CMD_RSPNS_TYPE_136;
const SD_RESP_R5: u32 = (SD_CMD_RSPNS_TYPE_48 | SD_CMD_CRCCHK_EN);
const SD_RESP_R5b: u32 = (SD_CMD_RSPNS_TYPE_48B | SD_CMD_CRCCHK_EN);
const SD_RESP_R6: u32 = (SD_CMD_RSPNS_TYPE_48 | SD_CMD_CRCCHK_EN);
const SD_RESP_R7: u32 = (SD_CMD_RSPNS_TYPE_48 | SD_CMD_CRCCHK_EN);

const SD_DATA_READ: u32 = (SD_CMD_ISDATA | SD_CMD_DAT_DIR_CH);
const SD_DATA_WRITE: u32 = (SD_CMD_ISDATA | SD_CMD_DAT_DIR_HC);

const SD_VER_UNKNOWN: u32 = 0;
const SD_VER_1: u32 = 1;
const SD_VER_1_1: u32 = 2;
const SD_VER_2: u32 = 3;
const SD_VER_3: u32 = 4;
const SD_VER_4: u32 = 5;

macro_rules! sd_cmd {
    (RESERVED, $index:expr) => {
        0xffff_ffff
    };
    (INDEX, $index:expr) => {
        ($index) << 24
    };
}

const sd_commands: [u32; 64] = [
    sd_cmd!(INDEX, 0),
    sd_cmd!(RESERVED, 1),
    sd_cmd!(INDEX, 2) | SD_RESP_R2,
    sd_cmd!(INDEX, 3) | SD_RESP_R6,
    sd_cmd!(INDEX, 4),
    sd_cmd!(INDEX, 5) | SD_RESP_R4,
    sd_cmd!(INDEX, 6) | SD_RESP_R1,
    sd_cmd!(INDEX, 7) | SD_RESP_R1b,
    sd_cmd!(INDEX, 8) | SD_RESP_R7,
    sd_cmd!(INDEX, 9) | SD_RESP_R2,
    sd_cmd!(INDEX, 10) | SD_RESP_R2,
    sd_cmd!(INDEX, 11) | SD_RESP_R1,
    sd_cmd!(INDEX, 12) | SD_RESP_R1b | SD_CMD_TYPE_ABORT,
    sd_cmd!(INDEX, 13) | SD_RESP_R1,
    sd_cmd!(RESERVED, 14),
    sd_cmd!(INDEX, 15),
    sd_cmd!(INDEX, 16) | SD_RESP_R1,
    sd_cmd!(INDEX, 17) | SD_RESP_R1 | SD_DATA_READ,
    sd_cmd!(INDEX, 18) | SD_RESP_R1 | SD_DATA_READ | SD_CMD_MULTI_BLOCK | SD_CMD_BLKCNT_EN,
    sd_cmd!(INDEX, 19) | SD_RESP_R1 | SD_DATA_READ,
    sd_cmd!(INDEX, 20) | SD_RESP_R1b,
    sd_cmd!(RESERVED, 21),
    sd_cmd!(RESERVED, 22),
    sd_cmd!(INDEX, 23) | SD_RESP_R1,
    sd_cmd!(INDEX, 24) | SD_RESP_R1 | SD_DATA_WRITE,
    sd_cmd!(INDEX, 25) | SD_RESP_R1 | SD_DATA_WRITE | SD_CMD_MULTI_BLOCK | SD_CMD_BLKCNT_EN,
    sd_cmd!(RESERVED, 26),
    sd_cmd!(INDEX, 27) | SD_RESP_R1 | SD_DATA_WRITE,
    sd_cmd!(INDEX, 28) | SD_RESP_R1b,
    sd_cmd!(INDEX, 29) | SD_RESP_R1b,
    sd_cmd!(INDEX, 30) | SD_RESP_R1 | SD_DATA_READ,
    sd_cmd!(RESERVED, 31),
    sd_cmd!(INDEX, 32) | SD_RESP_R1,
    sd_cmd!(INDEX, 33) | SD_RESP_R1,
    sd_cmd!(RESERVED, 34),
    sd_cmd!(RESERVED, 35),
    sd_cmd!(RESERVED, 36),
    sd_cmd!(RESERVED, 37),
    sd_cmd!(INDEX, 38) | SD_RESP_R1b,
    sd_cmd!(RESERVED, 39),
    sd_cmd!(RESERVED, 40),
    sd_cmd!(RESERVED, 41),
    sd_cmd!(RESERVED, 42) | SD_RESP_R1,
    sd_cmd!(RESERVED, 43),
    sd_cmd!(RESERVED, 44),
    sd_cmd!(RESERVED, 45),
    sd_cmd!(RESERVED, 46),
    sd_cmd!(RESERVED, 47),
    sd_cmd!(RESERVED, 48),
    sd_cmd!(RESERVED, 49),
    sd_cmd!(RESERVED, 50),
    sd_cmd!(RESERVED, 51),
    sd_cmd!(RESERVED, 52),
    sd_cmd!(RESERVED, 53),
    sd_cmd!(RESERVED, 54),
    sd_cmd!(INDEX, 55) | SD_RESP_R1,
    sd_cmd!(INDEX, 56) | SD_RESP_R1 | SD_CMD_ISDATA,
    sd_cmd!(RESERVED, 57),
    sd_cmd!(RESERVED, 58),
    sd_cmd!(RESERVED, 59),
    sd_cmd!(RESERVED, 60),
    sd_cmd!(RESERVED, 61),
    sd_cmd!(RESERVED, 62),
    sd_cmd!(RESERVED, 63),
];

const sd_acommands: [u32; 64] = [
    sd_cmd!(RESERVED, 0),
    sd_cmd!(RESERVED, 1),
    sd_cmd!(RESERVED, 2),
    sd_cmd!(RESERVED, 3),
    sd_cmd!(RESERVED, 4),
    sd_cmd!(RESERVED, 5),
    sd_cmd!(INDEX, 6) | SD_RESP_R1,
    sd_cmd!(RESERVED, 7),
    sd_cmd!(RESERVED, 8),
    sd_cmd!(RESERVED, 9),
    sd_cmd!(RESERVED, 10),
    sd_cmd!(RESERVED, 11),
    sd_cmd!(RESERVED, 12),
    sd_cmd!(INDEX, 13) | SD_RESP_R1,
    sd_cmd!(RESERVED, 14),
    sd_cmd!(RESERVED, 15),
    sd_cmd!(RESERVED, 16),
    sd_cmd!(RESERVED, 17),
    sd_cmd!(RESERVED, 18),
    sd_cmd!(RESERVED, 19),
    sd_cmd!(RESERVED, 20),
    sd_cmd!(RESERVED, 21),
    sd_cmd!(INDEX, 22) | SD_RESP_R1 | SD_DATA_READ,
    sd_cmd!(INDEX, 23) | SD_RESP_R1,
    sd_cmd!(RESERVED, 24),
    sd_cmd!(RESERVED, 25),
    sd_cmd!(RESERVED, 26),
    sd_cmd!(RESERVED, 27),
    sd_cmd!(RESERVED, 28),
    sd_cmd!(RESERVED, 29),
    sd_cmd!(RESERVED, 30),
    sd_cmd!(RESERVED, 31),
    sd_cmd!(RESERVED, 32),
    sd_cmd!(RESERVED, 33),
    sd_cmd!(RESERVED, 34),
    sd_cmd!(RESERVED, 35),
    sd_cmd!(RESERVED, 36),
    sd_cmd!(RESERVED, 37),
    sd_cmd!(RESERVED, 38),
    sd_cmd!(RESERVED, 39),
    sd_cmd!(RESERVED, 40),
    sd_cmd!(INDEX, 41) | SD_RESP_R3,
    sd_cmd!(INDEX, 42) | SD_RESP_R1,
    sd_cmd!(RESERVED, 43),
    sd_cmd!(RESERVED, 44),
    sd_cmd!(RESERVED, 45),
    sd_cmd!(RESERVED, 46),
    sd_cmd!(RESERVED, 47),
    sd_cmd!(RESERVED, 48),
    sd_cmd!(RESERVED, 49),
    sd_cmd!(RESERVED, 50),
    sd_cmd!(INDEX, 51) | SD_RESP_R1 | SD_DATA_READ,
    sd_cmd!(RESERVED, 52),
    sd_cmd!(RESERVED, 53),
    sd_cmd!(RESERVED, 54),
    sd_cmd!(RESERVED, 55),
    sd_cmd!(RESERVED, 56),
    sd_cmd!(RESERVED, 57),
    sd_cmd!(RESERVED, 58),
    sd_cmd!(RESERVED, 59),
    sd_cmd!(RESERVED, 60),
    sd_cmd!(RESERVED, 61),
    sd_cmd!(RESERVED, 62),
    sd_cmd!(RESERVED, 63),
];

const GO_IDLE_STATE: u32 = 0;
const ALL_SEND_CID: u32 = 2;
const SEND_RELATIVE_ADDR: u32 = 3;
const SET_DSR: u32 = 4;
const IO_SET_OP_COND: u32 = 5;
const SWITCH_FUNC: u32 = 6;
const SELECT_CARD: u32 = 7;
const DESELECT_CARD: u32 = 7;
const SELECT_DESELECT_CARD: u32 = 7;
const SEND_IF_COND: u32 = 8;
const SEND_CSD: u32 = 9;
const SEND_CID: u32 = 10;
const VOLTAGE_SWITCH: u32 = 11;
const STOP_TRANSMISSION: u32 = 12;
const SEND_STATUS: u32 = 13;
const GO_INACTIVE_STATE: u32 = 15;
const SET_BLOCKLEN: u32 = 16;
const READ_SINGLE_BLOCK: u32 = 17;
const READ_MULTIPLE_BLOCK: u32 = 18;
const SEND_TUNING_BLOCK: u32 = 19;
const SPEED_CLASS_CONTROL: u32 = 20;
const SET_BLOCK_COUNT: u32 = 23;
const WRITE_BLOCK: u32 = 24;
const WRITE_MULTIPLE_BLOCK: u32 = 25;
const PROGRAM_CSD: u32 = 27;
const SET_WRITE_PROT: u32 = 28;
const CLR_WRITE_PROT: u32 = 29;
const SEND_WRITE_PROT: u32 = 30;
const ERASE_WR_BLK_START: u32 = 32;
const ERASE_WR_BLK_END: u32 = 33;
const ERASE: u32 = 38;
const LOCK_UNLOCK: u32 = 42;
const APP_CMD: u32 = 55;
const GEN_CMD: u32 = 56;

const IS_APP_CMD: u32 = 0x80000000;
macro_rules! ACMD {
    ($a: expr) => {
        (($a) | (IS_APP_CMD))
    };
}
const SET_BUS_WIDTH: u32 = (6 | IS_APP_CMD);
const SD_STATUS: u32 = (13 | IS_APP_CMD);
const SEND_NUM_WR_BLOCKS: u32 = (22 | IS_APP_CMD);
const SET_WR_BLK_ERASE_COUNT: u32 = (23 | IS_APP_CMD);
const SD_SEND_OP_COND: u32 = (41 | IS_APP_CMD);
const SET_CLR_CARD_DETECT: u32 = (42 | IS_APP_CMD);
const SEND_SCR: u32 = (51 | IS_APP_CMD);

const SD_RESET_CMD: u32 = (1 << 25);
const SD_RESET_DAT: u32 = (1 << 26);
const SD_RESET_ALL: u32 = (1 << 24);

#[repr(C)]
#[derive(Debug)]
pub struct SDScr {
    scr: [u32; 2],
    sd_bus_widths: u32,
    sd_version: u32,
}

impl SDScr {
    pub fn new() -> SDScr {
        SDScr {
            scr: [0u32; 2],
            sd_bus_widths: 0,
            sd_version: 0,
        }
    }
}

pub struct EmmcCtl {
    emmc: Emmc,
    card_supports_sdhc: bool,
    card_supports_18v: bool,
    card_ocr: u32,
    card_rca: u32,
    last_interrupt: u32,
    last_error: u32,

    sd_scr: SDScr,
    failed_voltage_switch: i32,

    last_cmd_reg: u32,
    last_cmd: u32,
    last_cmd_success: bool,
    last_r: [u32; 4],

    // void *buf;
    blocks_to_transfer: u32,
    block_size: usize,
    use_sdma: bool,
    card_removal: bool,
    base_clock: u32,
}

fn usleep(cnt: usize) {
    bcm2837::timer::delay_us(cnt);
}

fn byte_swap(b: u32) -> u32 {
    (b >> 24) | ((b & 0xFF0000) >> 8) | ((b & 0xFF00) << 8) | ((b & 0xFF) << 24)
}

/*
 * TODO:
 * ++ static void sd_power_off()
 * static uint32_t sd_get_base_clock_hz()
 * -- static int bcm_2708_power_off()
 * -- static int bcm_2708_power_on()
 * -- static int bcm_2708_power_cycle()
 * ++ static uint32_t sd_get_clock_divider(uint32_t base_clock, uint32_t target_rate)
 * ++ static int sd_switch_clock_rate(uint32_t base_clock, uint32_t target_rate)
 * ++ static int sd_reset_cmd()
 * ++ static int sd_reset_dat()
 * ++ static void sd_issue_command_int(struct emmc_block_dev *dev, uint32_t cmd_reg, uint32_t argument, useconds_t timeout)
 * static void sd_handle_card_interrupt(struct emmc_block_dev *dev)
 * static void sd_handle_interrupts(struct emmc_block_dev *dev)
 *
 *
 * ++ static void sd_issue_command(struct emmc_block_dev *dev, uint32_t command, uint32_t argument, useconds_t timeout)
 * ++ static int sd_ensure_data_mode(struct emmc_block_dev *edev)
 * -- static int sd_suitable_for_dma(void *buf)
 * -- static int sd_do_data_command(struct emmc_block_dev *edev, int is_write, uint8_t *buf, size_t buf_size, uint32_t block_no)
 * ++ int sd_card_init(struct block_device **dev)
 * ++ int sd_read(struct block_device *dev, uint8_t *buf, size_t buf_size, uint32_t block_no)
 * ++ int sd_write(struct block_device *dev, uint8_t *buf, size_t buf_size, uint32_t block_no)
 * Other Constants
 */

const MAX_WAIT_US: u32 = 1000000;
const MAX_WAIT_TIMES: u32 = MAX_WAIT_US / 1000;

macro_rules! timeout_wait {
    ($condition:expr) => {{
        let mut succeeded = false;
        for _ in 0..MAX_WAIT_TIMES {
            if $condition {
                succeeded = true;
                break;
            }
            usleep(1000);
        }
        succeeded
    }};

    ($condition:expr, $timeout:expr) => {{
        let mut succeeded = false;
        for _ in 0..(($timeout) / 1000) {
            if $condition {
                succeeded = true;
                break;
            }
            usleep(1000);
        }
        succeeded
    }};
}

impl EmmcCtl {
    pub fn new() -> EmmcCtl {
        //TODO: improve it!
        EmmcCtl {
            emmc: Emmc::new(),
            card_supports_sdhc: false,
            card_supports_18v: false,
            card_ocr: 0,
            card_rca: 0,
            last_interrupt: 0,
            last_error: 0,

            sd_scr: SDScr::new(),
            failed_voltage_switch: 0,

            last_cmd_reg: 0,
            last_cmd: 0,
            last_cmd_success: false,
            last_r: [0; 4],

            blocks_to_transfer: 0,
            block_size: 0,
            use_sdma: false,
            card_removal: false,
            base_clock: 0,
        }
    }

    fn succeeded(&self) -> bool {
        return self.last_cmd_success;
    }
    fn failed(&self) -> bool {
        return !self.last_cmd_success;
    }
    fn timeout(&self) -> bool {
        return self.failed() && self.last_error == 0;
    }
    fn cmd_timeout(&self) -> bool {
        return self.failed() && (self.last_error & (1 << SD_ERR_BASE + SD_ERR_CMD_TIMEOUT)) != 0;
    }
    fn cmd_crc(&self) -> bool {
        return self.failed() && (self.last_error & (1 << SD_ERR_BASE + SD_ERR_CMD_CRC)) != 0;
    }
    fn cmd_end_bit(&self) -> bool {
        return self.failed() && (self.last_error & (1 << SD_ERR_BASE + SD_ERR_CMD_END_BIT)) != 0;
    }
    fn cmd_index(&self) -> bool {
        return self.failed() && (self.last_error & (1 << SD_ERR_BASE + SD_ERR_CMD_INDEX)) != 0;
    }
    fn data_timeout(&self) -> bool {
        return self.failed() && (self.last_error & (1 << SD_ERR_BASE + SD_ERR_DATA_TIMEOUT)) != 0;
    }
    fn data_crc(&self) -> bool {
        return self.failed() && (self.last_error & (1 << SD_ERR_BASE + SD_ERR_DATA_CRC)) != 0;
    }
    fn data_end_bit(&self) -> bool {
        return self.failed() && (self.last_error & (1 << SD_ERR_BASE + SD_ERR_DATA_END_BIT)) != 0;
    }
    fn current_limit(&self) -> bool {
        return self.failed() && (self.last_error & (1 << SD_ERR_BASE + SD_ERR_CURRENT_LIMIT)) != 0;
    }
    fn acmd12_error(&self) -> bool {
        return self.failed() && (self.last_error & (1 << SD_ERR_BASE + SD_ERR_AUTO_CMD12)) != 0;
    }
    fn adma_error(&self) -> bool {
        return self.failed() && (self.last_error & (1 << SD_ERR_BASE + SD_ERR_ADMA)) != 0;
    }
    fn tuning_error(&self) -> bool {
        return self.failed() && (self.last_error & (1 << SD_ERR_BASE + SD_ERR_TUNING)) != 0;
    }
    pub fn sd_power_off(&mut self) {
        let ctl0 = self.emmc.registers.CONTROL0.read();
        self.emmc.registers.CONTROL0.write(ctl0 & !(1 << 8));
    }

    pub fn sd_get_base_clock_hz(&mut self) -> u32 {
        let buf = mailbox::get_clock_rate(0x1);
        if buf.is_ok() {
            let base_clock = buf.unwrap();
            debug!("EmmcCtl: base clock rate is {}Hz.", base_clock);
            return base_clock;
        } else {
            warn!("EmmcCtl: property mailbox did not return a valid clock id.");
            return 0;
        }
    }

    pub fn sd_get_clock_divider(&mut self, base_clock: u32, target_rate: u32) -> u32 {
        let targetted_divisor: u32 = if (target_rate > base_clock) {
            1
        } else {
            base_clock / target_rate
                - if (base_clock % target_rate != 0) {
                    1
                } else {
                    0
                }
        };

        let mut divisor = 31;

        for first_bit in (0..32).rev() {
            if targetted_divisor & (1 << first_bit) != 0 {
                divisor = first_bit
                    + if targetted_divisor != (1 << first_bit) {
                        1
                    } else {
                        0
                    };
                break;
            }
        }

        if divisor >= 32 {
            divisor = 31;
        }

        if divisor != 0 {
            divisor = 1 << (divisor - 1);
        }

        if divisor >= 0x400 {
            divisor = 0x3ff;
        }

        ((divisor & 0xff) << 8) | (((divisor >> 8) & 0x3) << 6) | (0 << 5)
    }

    pub fn sd_switch_clock_rate(&mut self, base_clock: u32, target_rate: u32) -> bool {
        let divider = self.sd_get_clock_divider(base_clock, target_rate);

        // Wait for the command inhibit (CMD and DAT) bits to clear
        loop {
            if self.emmc.registers.STATUS.read() & 0x3 == 0 {
                break;
            }

            usleep(1000);
        }

        // Set the SD clock off
        let mut control1 = self.emmc.registers.CONTROL1.read();
        control1 &= !(1 << 2);
        self.emmc.registers.CONTROL1.write(control1);
        usleep(2000);

        // Write the new divider
        control1 &= !0xffe0; // Clear old setting + clock generator select
        control1 |= divider;
        self.emmc.registers.CONTROL1.write(control1);
        usleep(2000);

        // Enable the SD clock
        control1 |= (1 << 2);
        self.emmc.registers.CONTROL1.write(control1);
        usleep(2000);

        true
    }

    pub fn sd_reset_cmd(&mut self) -> bool {
        let mut control1 = self.emmc.registers.CONTROL1.read();
        self.emmc.registers.CONTROL1.write(control1 | SD_RESET_CMD);

        timeout_wait!(self.emmc.registers.CONTROL1.read() & SD_RESET_CMD == 0);
        if self.emmc.registers.CONTROL1.read() & SD_RESET_CMD != 0 {
            warn!("EmmcCtl: CMD line did not reset properly.");
            return false;
        }
        true
    }

    pub fn sd_reset_dat(&mut self) -> bool {
        let mut control1 = self.emmc.registers.CONTROL1.read();
        self.emmc.registers.CONTROL1.write(control1 | SD_RESET_DAT);

        timeout_wait!(self.emmc.registers.CONTROL1.read() & SD_RESET_DAT == 0);
        if self.emmc.registers.CONTROL1.read() & SD_RESET_DAT != 0 {
            warn!("EmmcCtl: DAT line did not reset properly.");
            return false;
        }
        true
    }

    pub fn sd_handle_card_interrupt(&mut self) {
        if self.card_rca != 0 {
            self.sd_issue_command_int(
                sd_commands[SEND_STATUS as usize],
                self.card_rca << 16,
                500000,
            );
        }
    }

    pub fn sd_handle_interrupts(&mut self) {
        let irpts = self.emmc.registers.INTERRUPT.read();
        let mut reset_mask = 0;

        if irpts & SD_COMMAND_COMPLETE != 0 {
            reset_mask |= SD_COMMAND_COMPLETE;
        }

        if irpts & SD_TRANSFER_COMPLETE != 0 {
            reset_mask |= SD_TRANSFER_COMPLETE;
        }

        if irpts & SD_BLOCK_GAP_EVENT != 0 {
            reset_mask |= SD_BLOCK_GAP_EVENT;
        }

        if irpts & SD_DMA_INTERRUPT != 0 {
            reset_mask |= SD_DMA_INTERRUPT;
        }

        if irpts & SD_BUFFER_WRITE_READY != 0 {
            reset_mask |= SD_BUFFER_WRITE_READY;
            self.sd_reset_dat();
        }

        if irpts & SD_BUFFER_READ_READY != 0 {
            reset_mask |= SD_BUFFER_READ_READY;
            self.sd_reset_dat();
        }

        if irpts & SD_CARD_INSERTION != 0 {
            reset_mask |= SD_CARD_INSERTION;
        }

        if irpts & SD_CARD_REMOVAL != 0 {
            reset_mask |= SD_CARD_REMOVAL;
            self.card_removal = true;
        }

        if irpts & SD_CARD_INTERRUPT != 0 {
            self.sd_handle_card_interrupt();
            reset_mask |= SD_CARD_INTERRUPT;
        }

        if irpts & 0x8000 != 0 {
            reset_mask |= 0xffff0000;
        }

        self.emmc.registers.INTERRUPT.write(reset_mask);
    }

    pub fn sd_issue_command_int_pre(&mut self, command: u32, argument: u32, timeout: u32) -> bool {
        self.last_cmd_reg = command;
        self.last_cmd_success = false;

        while (self.emmc.registers.STATUS.read() & 0x1) != 0 {
            usleep(1000);
        }

        if ((command & SD_CMD_RSPNS_TYPE_MASK) == SD_CMD_RSPNS_TYPE_48B)
            && ((command & SD_CMD_TYPE_MASK) == SD_CMD_TYPE_ABORT)
        {
            while (self.emmc.registers.STATUS.read() & 0x2) != 0 {
                usleep(1000);
            }
        }

        if self.blocks_to_transfer > 0xffff {
            self.last_cmd_success = false;
            return false;
        }
        let blksizecnt = self.block_size as u32 | (self.blocks_to_transfer << 16);
        self.emmc.registers.BLKSIZECNT.write(blksizecnt);
        self.emmc.registers.ARG1.write(argument);
        self.emmc.registers.CMDTM.write(command);
        usleep(2000);

        timeout_wait!(self.emmc.registers.INTERRUPT.read() & 0x8001 != 0, timeout);

        let irpts = self.emmc.registers.INTERRUPT.read();
        self.emmc.registers.INTERRUPT.write(0xffff_0001);
        if (irpts & 0xffff_0001) != 0x1 {
            self.last_error = irpts & 0xffff_0000;
            self.last_interrupt = irpts;
            return false;
        }
        usleep(2000);
        if command & SD_CMD_RSPNS_TYPE_MASK == SD_CMD_RSPNS_TYPE_48
            || command & SD_CMD_RSPNS_TYPE_MASK == SD_CMD_RSPNS_TYPE_48B
        {
            self.last_r[0] = self.emmc.registers.RESP[0].read();
        } else {
            for i in 0..3 {
                self.last_r[i] = self.emmc.registers.RESP[i].read();
            }
        }
        true
    }

    pub fn sd_issue_command_int_post(&mut self, command: u32, argument: u32, timeout: u32) -> bool {
        if ((command & SD_CMD_RSPNS_TYPE_MASK) == SD_CMD_RSPNS_TYPE_48B)
            || (command & SD_CMD_ISDATA) != 0
        {
            if (self.emmc.registers.STATUS.read() & 0x2) == 0 {
                self.emmc.registers.INTERRUPT.write(0xffff_0002);
            } else {
                timeout_wait!(
                    (self.emmc.registers.INTERRUPT.read() & 0x8002) != 0,
                    timeout
                );
                let mut irpts = self.emmc.registers.INTERRUPT.read();
                self.emmc.registers.INTERRUPT.write(0xffff_0002);

                if (irpts & 0xffff_0002) != 0x2 && (irpts & 0xffff_0002) != 0x10_0002 {
                    warn!("EmmcCtl: error occurred while waiting for transfer complete interrupt.");
                    self.last_error = irpts & 0xffff_0000;
                    self.last_interrupt = irpts;
                    return false;
                }

                self.emmc.registers.INTERRUPT.write(0xffff_0002);
            }
        }
        self.last_cmd_success = true;
        true
    }

    pub fn sd_issue_command_int(&mut self, command: u32, argument: u32, timeout: u32) {
        if self.sd_issue_command_int_pre(command, argument, timeout) {
            self.sd_issue_command_int_post(command, argument, timeout);
        }
    }

    pub fn sd_issue_command(&mut self, command: u32, argument: u32, timeout: u32) -> bool {
        self.sd_handle_interrupts();
        if command & IS_APP_CMD != 0 {
            let cmd = command & 0xff;
            if sd_acommands[cmd as usize] == sd_cmd!(RESERVED, 0) {
                self.last_cmd_success = false;
                return false;
            }
            self.last_cmd = APP_CMD;

            let mut rca = 0;
            if self.card_rca != 0 {
                rca = self.card_rca << 16;
            }
            self.sd_issue_command_int(sd_commands[APP_CMD as usize], rca, timeout);
            if self.last_cmd_success {
                self.last_cmd = cmd | IS_APP_CMD;
                self.sd_issue_command_int(sd_acommands[cmd as usize], argument, timeout);
            }
        } else {
            if sd_commands[command as usize] == sd_cmd!(RESERVED, 0) {
                self.last_cmd_success = false;
                return false;
            }
            self.last_cmd = command;
            self.sd_issue_command_int(sd_commands[command as usize], argument, timeout);
        }
        self.last_cmd_success
    }

    pub fn sd_issue_command_scr(&mut self, timeout: u32) -> bool {
        self.sd_handle_interrupts();
        let command = SEND_SCR;
        let cmd = command & 0xff;

        let mut rca = 0;
        let count = 1;
        let blocks_size_u32 = self.block_size / 4;

        self.last_cmd = APP_CMD;
        if self.card_rca != 0 {
            rca = self.card_rca << 16;
        }
        self.sd_issue_command_int(sd_commands[APP_CMD as usize], rca, timeout);

        if self.last_cmd_success {
            self.last_cmd = command;
            let command = sd_acommands[cmd as usize];
            debug!(
                "EmmcCtl: block_size = {}, blocks_to_transfer = {}.",
                self.block_size, self.blocks_to_transfer
            );
            if self.sd_issue_command_int_pre(command, 0, timeout) {
                let mut buf = &mut self.sd_scr.scr;
                let mut wr_irpt = (1 << 5);
                let mut finished = true;
                for cur_block in 0..count {
                    timeout_wait!(
                        self.emmc.registers.INTERRUPT.read() & (wr_irpt | 0x8000) != 0,
                        timeout
                    );
                    let irpts = self.emmc.registers.INTERRUPT.read();
                    self.emmc.registers.INTERRUPT.write(0xffff_0000 | wr_irpt);
                    if (irpts & (0xffff_0000 | wr_irpt)) != wr_irpt {
                        warn!("EmmcCtl: error occurred while waiting for data block #{} ready interrupt.", count);
                        self.last_error = irpts & 0xffff_0000;
                        self.last_interrupt = irpts;
                        finished = false;
                        break;
                    }
                    let mut cur_word_no = 0;
                    while (cur_word_no < blocks_size_u32) {
                        let word = self.emmc.registers.DATA.read();
                        debug!(
                            "EmmcCtl: block#{}, word#{} = 0x{:08X}, pos = {}",
                            cur_block,
                            cur_word_no,
                            word,
                            (cur_block as usize) * blocks_size_u32 + cur_word_no
                        );
                        buf[(cur_block as usize) * blocks_size_u32 + cur_word_no] = word;
                        //self.emmc.registers.DATA.read();
                        cur_word_no += 1;
                    }
                }
                if finished {
                    self.sd_issue_command_int_post(command, 0, timeout);
                }
            }
            if self.last_cmd_success {
                return true;
            }
        }
        self.last_cmd_success
    }

    pub fn sd_check_success(&mut self) -> bool {
        if self.last_cmd_success {
            return true;
        } else {
            self.card_rca = 0;
            return false;
        }
    }

    pub fn sd_ensure_data_mode(&mut self) -> i32 {
        if self.card_rca == 0 {
            let ret = self.init();
            if ret != 0 {
                return ret;
            }
        }

        self.sd_issue_command(SEND_STATUS, self.card_rca << 16, 500000);
        if !self.sd_check_success() {
            return -1;
        }

        let cur_state = (self.last_r[0] >> 9) & 0xf;

        if cur_state == 3 {
            self.sd_issue_command(SELECT_CARD, self.card_rca << 16, 500000);
            if !self.sd_check_success() {
                return -1;
            }
        } else if cur_state == 5 {
            self.sd_issue_command(STOP_TRANSMISSION, 0, 500000);
            if !self.sd_check_success() {
                return -1;
            }
            self.sd_reset_dat();
        } else if cur_state != 4 {
            let ret = self.init();
            if ret != 0 {
                return ret;
            }
        }

        if cur_state != 4 {
            self.sd_issue_command(SEND_STATUS, self.card_rca << 16, 500000);
            if !self.sd_check_success() {
                return -1;
            }

            let cur_state = (self.last_r[0] >> 9) & 0xf;
            if cur_state != 4 {
                self.card_rca = 0;
                return -1;
            }
        }
        0
    }

    pub fn sd_card_init(&mut self) -> bool {
        let ver = self.emmc.registers.SLOTISR_VER.read();
        let vendor = ver >> 24;
        let sdversion = (ver >> 16) & 0xff;
        let slot_status = ver & 0xff;
        debug!("EmmcCtl: vendor version number: {}", vendor);
        debug!("EmmcCtl: host controller version number: {}", sdversion);
        debug!("EmmcCtl: slot status: 0b{:b}", slot_status);

        let mut control0 = self.emmc.registers.CONTROL0.read();
        let mut control1 = self.emmc.registers.CONTROL1.read();
        control1 |= (1 << 24);
        // Disable clock
        control1 &= !(1 << 2);
        control1 &= !(1 << 0);
        self.emmc.registers.CONTROL1.write(control1);

        if !timeout_wait!((self.emmc.registers.CONTROL1.read() & (0x7 << 24)) == 0) {
            return false;
        }

        debug!("EmmcCtl: checking for a valid card");
        let tmp_status = self.emmc.registers.STATUS.read();
        debug!(
            "EmmcCtl: try to get current status, status = 0x{:X}",
            tmp_status
        );
        // Check for a valid card
        if !timeout_wait!((self.emmc.registers.STATUS.read() & (1 << 16)) != 0, 500000) {
            return false;
        }

        // Clear control2
        self.emmc.registers.CONTROL2.write(0);
        let clk = self.sd_get_base_clock_hz();
        let base_clock = if clk > 0 { clk } else { 100000000 };

        // Set clock rate to something slow
        control1 = self.emmc.registers.CONTROL1.read();
        control1 |= 1;

        let f_id = self.sd_get_clock_divider(base_clock, 400000);
        control1 |= f_id;

        control1 |= (7 << 16); // data timeout = TMCLK * 2^10

        self.emmc.registers.CONTROL1.write(control1);

        timeout_wait!(self.emmc.registers.CONTROL1.read() & 0x2 != 0, 0x1000000);
        if self.emmc.registers.CONTROL1.read() & 0x2 == 0 {
            warn!("EmmcCtl: controller's clock did not stabilize within 1 second.");
            return false;
        }

        // Enable the SD clock
        usleep(2000);
        control1 = self.emmc.registers.CONTROL1.read();
        control1 |= 4;
        self.emmc.registers.CONTROL1.write(control1);
        usleep(2000);

        // Mask off sending interrupts to the ARM
        self.emmc.registers.IRPT_EN.write(0);
        // Reset interrupts
        self.emmc.registers.INTERRUPT.write(0xffffffff);
        // Have all interrupts sent to the INTERRUPT register
        let irpt_mask = 0xffffffff & (!SD_CARD_INTERRUPT);
        self.emmc.registers.IRPT_MASK.write(0xffffffff);

        usleep(2000);

        self.block_size = 512;
        self.base_clock = base_clock;

        // Send CMD0 to the card (reset to idle state)
        debug!("EmmcCtl: Send CMD0 to the card (reset to idle state).");
        if !self.sd_issue_command(GO_IDLE_STATE, 0, 500000) {
            warn!("EmmcCtl: no CMD0 response.");
            return false;
        }

        // Send CMD8 to the card
        // Voltage supplied = 0x1 = 2.7-3.6V (standard)
        // Check pattern = 10101010b (as per PLSS 4.3.13) = 0xAA
        self.sd_issue_command(SEND_IF_COND, 0x1aa, 500000);
        let v2_later = if self.timeout() {
            false
        } else if self.cmd_timeout() {
            if !self.sd_reset_cmd() {
                return false;
            }
            self.emmc.registers.INTERRUPT.write(SD_ERR_MASK_CMD_TIMEOUT);
            false
        } else if self.failed() {
            warn!(
                "EmmcCtl: failure sending CMD8 (0x{:08X}).",
                self.last_interrupt
            );
            return false;
        } else {
            if self.last_r[0] & 0xfff != 0x1aa {
                warn!("EmmcCtl: unusable card.");
                return false;
            }
            true
        };

        // Here we are supposed to check the response to CMD5 (HCSS 3.6)
        // It only returns if the card is a SDIO card
        self.sd_issue_command(IO_SET_OP_COND, 0, 10000);
        if !self.timeout() {
            if self.cmd_timeout() {
                if !self.sd_reset_cmd() {
                    return false;
                }
                self.emmc.registers.INTERRUPT.write(SD_ERR_MASK_CMD_TIMEOUT);
            } else {
                warn!("EmmcCtl: SDIO card detected - not currently supported.");
                return false;
            }
        }

        if !self.sd_issue_command(ACMD!(41), 0, 500000) {
            warn!("EmmcCtl: inquiry ACMD41 failed.");
            return false;
        }

        let mut card_is_busy = true;

        while card_is_busy {
            let v2_flags = if v2_later {
                (1 << 30)
                    | if self.failed_voltage_switch == 0 {
                        (1 << 24)
                    } else {
                        0
                    }
            } else {
                0
            };
            if !self.sd_issue_command(ACMD!(41), 0x00ff8000 | v2_flags, 500000) {
                warn!("EmmcCtl: error issuing ACMD41.");
                return false;
            }

            if (self.last_r[0] >> 31) & 0x1 != 0 {
                self.card_ocr = (self.last_r[0] >> 8) & 0xffff;
                self.card_supports_sdhc = (self.last_r[0] >> 30) & 0x1 != 0;

                if self.failed_voltage_switch == 0 {
                    self.card_supports_18v = (self.last_r[0] >> 24) & 0x1 != 0;
                }

                card_is_busy = false;
            } else {
                usleep(500000);
            }
        }

        // At this point, we know the card is definitely an SD card, so will definitely
        //  support SDR12 mode which runs at 25 MHz
        self.sd_switch_clock_rate(base_clock, 25000000 /* SD_CLOCK_NORMAL */);

        // A small wait before the voltage switch
        usleep(5000);

        // Switch to 1.8V mode if possible
        debug!("EmmcCtl: card_supports_18v = {}", self.card_supports_18v);
        if (self.card_supports_18v) {
            // As per HCSS 3.6.1
            debug!("EmmcCtl: Switch to 1.8v mode.");
            // Send VOLTAGE_SWITCH
            if !self.sd_issue_command(VOLTAGE_SWITCH, 0, 500000) {
                self.failed_voltage_switch = 1;
                self.sd_power_off();
                return self.sd_card_init();
            }

            // Disable SD clock
            control1 = self.emmc.registers.CONTROL1.read();
            control1 &= !(1 << 2);
            self.emmc.registers.CONTROL1.write(control1);

            // Check DAT[3:0]
            let status_reg = self.emmc.registers.STATUS.read();
            if ((status_reg >> 20) & 0xf) != 0 {
                self.failed_voltage_switch = 1;
                self.sd_power_off();
                return self.sd_card_init();
            }

            // Set 1.8V signal enable to 1
            control0 = self.emmc.registers.CONTROL0.read();
            control0 |= (1 << 8);
            self.emmc.registers.CONTROL0.write(control0);

            usleep(5000);

            // Check the 1.8V signal enable is set
            control0 = self.emmc.registers.CONTROL0.read();
            if ((control0 >> 8) & 0x1) == 0 {
                self.failed_voltage_switch = 1;
                self.sd_power_off();
                return self.sd_card_init();
            }

            // Re-enable SD clock
            control1 = self.emmc.registers.CONTROL1.read();
            control1 |= (1 << 2);
            self.emmc.registers.CONTROL1.write(control1);

            // Wait 1 ms
            usleep(10000);

            // Check DAT[3:0]
            let status_reg = self.emmc.registers.STATUS.read();
            if ((status_reg >> 20) & 0xf) != 0xf {
                self.failed_voltage_switch = 1;
                self.sd_power_off();
                return self.sd_card_init();
            }
        }

        if !self.sd_issue_command(ALL_SEND_CID, 0, 500000) {
            return false;
        }

        debug!(
            "EmmcCtl: card CID: {:08X}{:08X}{:08X}{:08X}",
            self.last_r[0], self.last_r[1], self.last_r[2], self.last_r[3]
        );

        let card_cid = [
            self.last_r[0],
            self.last_r[1],
            self.last_r[2],
            self.last_r[3],
        ];

        if !self.sd_issue_command(SEND_RELATIVE_ADDR, 0, 500000) {
            return false;
        }

        let cmd3_resp = self.last_r[0];
        self.card_rca = (cmd3_resp >> 16) & 0xffff;

        if (cmd3_resp >> 15) & 0x1 != 0 {
            // CRC error
            warn!("EmmcCtl: CRC error.");
            return false;
        }

        if (cmd3_resp >> 14) & 0x1 != 0 {
            // illegal command
            warn!("EmmcCtl: Illegal command.");
            return false;
        }

        if (cmd3_resp >> 13) & 0x1 != 0 {
            // generic error
            warn!("EmmcCtl: Generic error.");
            return false;
        }

        if (cmd3_resp >> 8) & 0x1 == 0 {
            // not ready for data
            warn!("EmmcCtl: Not ready for data.");
            return false;
        }

        // Now select the card (toggles it to transfer state)
        debug!("EmmcCtl: Toggle the card to transfer state.");
        if !self.sd_issue_command(SELECT_CARD, self.card_rca << 16, 500000) {
            warn!("EmmcCtl: Error sending SELECT_CARD.");
            return false;
        }

        let cmd7_resp = self.last_r[0];
        let status = (cmd7_resp >> 9) & 0xf;

        if status != 3 && status != 4 {
            warn!("EmmcCtl: Invalid status ({}).", status);
            return false;
        }

        // If not an SDHC card, ensure BLOCKLEN is 512 bytes
        debug!("EmmcCtl: card_supports_sdhc = {}.", self.card_supports_sdhc);
        if !self.card_supports_sdhc {
            if !self.sd_issue_command(SET_BLOCKLEN, 512, 500000) {
                return false;
            }
        }

        self.block_size = 512;
        let mut controller_block_size = self.emmc.registers.BLKSIZECNT.read();
        controller_block_size &= (!0xfff);
        controller_block_size |= 0x200;
        self.emmc.registers.BLKSIZECNT.write(controller_block_size);

        self.block_size = 8;
        self.blocks_to_transfer = 1;

        /*
        if !self.sd_issue_command(SEND_SCR, 0, 500000) {
            self.block_size = 512;
            return false;
        }
        */

        if !self.sd_issue_command_scr(500000) {
            warn!("EmmcCtl: Error sending SEND_SCR.");
            self.block_size = 512;
            return false;
        }

        self.block_size = 512;

        // Determine card version
        // Note that the SCR is big-endian
        debug!("EmmcCtl: Check the card version.");
        let scr0 = byte_swap(self.sd_scr.scr[0]);
        self.sd_scr.sd_version = SD_VER_UNKNOWN;
        let sd_spec = (scr0 >> (56 - 32)) & 0xf;
        let sd_spec3 = (scr0 >> (47 - 32)) & 0x1;
        let sd_spec4 = (scr0 >> (42 - 32)) & 0x1;
        self.sd_scr.sd_bus_widths = (scr0 >> (48 - 32)) & 0xf;
        self.sd_scr.sd_version = if sd_spec == 0 {
            SD_VER_1
        } else if sd_spec == 1 {
            SD_VER_1_1
        } else if sd_spec == 2 {
            if sd_spec3 == 0 {
                SD_VER_2
            } else if sd_spec3 == 1 {
                if sd_spec4 == 0 {
                    SD_VER_3
                } else if sd_spec4 == 1 {
                    SD_VER_4
                } else {
                    SD_VER_UNKNOWN
                }
            } else {
                SD_VER_UNKNOWN
            }
        } else {
            SD_VER_UNKNOWN
        };

        fn sd_version(version: u32) -> &'static str {
            match version {
                SD_VER_UNKNOWN => "unknown",
                SD_VER_1 => "1.0 and 1.01",
                SD_VER_1_1 => "1.10",
                SD_VER_2 => "2.00",
                SD_VER_3 => "3.0x",
                SD_VER_4 => "4.0x",
                _ => "unknown",
            }
        }

        debug!(
            "EmmcCtl: SCR: version {}, bus_widths 0x{:X}",
            sd_version(self.sd_scr.sd_version),
            self.sd_scr.sd_bus_widths
        );

        if self.sd_scr.sd_bus_widths & 0x4 != 0 {
            // Set 4-bit transfer mode (ACMD6)
            // See HCSS 3.4 for the algorithm

            // Disable card interrupt in host
            let old_irpt_mask = self.emmc.registers.IRPT_MASK.read();
            let new_irpt_mask = old_irpt_mask & !(1 << 8);
            self.emmc.registers.IRPT_MASK.write(new_irpt_mask);

            // Send ACMD6 to change the card's bit mode
            if self.sd_issue_command(SET_BUS_WIDTH, 0x2, 500000) {
                // Change bit mode for Host
                control0 = self.emmc.registers.CONTROL0.read();
                control0 |= 0x2;
                self.emmc.registers.CONTROL0.write(control0);

                // Re-enable card interrupt in host
                self.emmc.registers.IRPT_MASK.write(old_irpt_mask);
            }
        }

        // Reset interrupt register
        self.emmc.registers.INTERRUPT.write(0xffffffff);

        true
    }

    pub fn read(&mut self, block_no_arg: u32, count: usize, buf: &mut [u32]) -> Result<(), ()> {
        let mut block_no = block_no_arg;
        if !self.card_supports_sdhc {
            block_no *= 512;
        }
        //assert_eq!(count * self.block_size, buf.len());
        self.blocks_to_transfer = count as u32;
        let mut command = 0;
        let blocks_size_u32 = self.block_size / 4;
        if count > 1 {
            command = READ_MULTIPLE_BLOCK;
        } else {
            command = READ_SINGLE_BLOCK;
        }
        for retry in 0..3 {
            {
                // send command
                self.last_cmd = command;
                if self.sd_issue_command_int_pre(sd_commands[command as usize], block_no, 500000) {
                    let mut wr_irpt = (1 << 5);
                    let mut finished = true;
                    for cur_block in 0..count {
                        timeout_wait!(
                            self.emmc.registers.INTERRUPT.read() & (wr_irpt | 0x8000) != 0,
                            500000
                        );
                        let irpts = self.emmc.registers.INTERRUPT.read();
                        self.emmc.registers.INTERRUPT.write(0xffff_0000 | wr_irpt);
                        if (irpts & (0xffff_0000 | wr_irpt)) != wr_irpt {
                            self.last_error = irpts & 0xffff_0000;
                            self.last_interrupt = irpts;
                            finished = false;
                            break;
                        }
                        let mut cur_word_no = 0;
                        while (cur_word_no < blocks_size_u32) {
                            buf[(cur_block as usize) * blocks_size_u32 + cur_word_no] =
                                self.emmc.registers.DATA.read();
                            cur_word_no += 1;
                        }
                    }
                    if finished {
                        self.sd_issue_command_int_post(
                            sd_commands[command as usize],
                            block_no,
                            500000,
                        );
                    }
                }
                if self.last_cmd_success {
                    return Ok(());
                }
            }
        }
        self.card_rca = 0;
        Err(())
    }

    pub fn write(&mut self, block_no_arg: u32, count: usize, buf: &[u32]) -> Result<(), ()> {
        let mut block_no = block_no_arg;
        if !self.card_supports_sdhc {
            block_no *= 512;
        }
        //assert_eq!(count * self.block_size, buf.len());
        self.blocks_to_transfer = count as u32;
        let mut command = 0;
        let blocks_size_u32 = self.block_size / 4;
        if count > 1 {
            command = WRITE_MULTIPLE_BLOCK;
        } else {
            command = WRITE_BLOCK;
        }
        for retry in 0..3 {
            {
                // send command
                self.last_cmd = command;
                if self.sd_issue_command_int_pre(sd_commands[command as usize], block_no, 500000) {
                    let mut wr_irpt = (1 << 4);
                    let mut finished = true;
                    for cur_block in 0..count {
                        timeout_wait!(
                            self.emmc.registers.INTERRUPT.read() & (wr_irpt | 0x8000) != 0,
                            500000
                        );
                        let irpts = self.emmc.registers.INTERRUPT.read();
                        self.emmc.registers.INTERRUPT.write(0xffff_0000 | wr_irpt);
                        if (irpts & (0xffff_0000 | wr_irpt)) != wr_irpt {
                            self.last_error = irpts & 0xffff_0000;
                            self.last_interrupt = irpts;
                            finished = false;
                            break;
                        }
                        let mut cur_word_no = 0;
                        while (cur_word_no < blocks_size_u32) {
                            self.emmc
                                .registers
                                .DATA
                                .write(buf[(cur_block as usize) * blocks_size_u32 + cur_word_no]);
                            cur_word_no += 1;
                        }
                    }
                    if finished {
                        self.sd_issue_command_int_post(
                            sd_commands[command as usize],
                            block_no,
                            500000,
                        );
                    }
                }
                if self.last_cmd_success {
                    return Ok(());
                }
            }
        }
        self.card_rca = 0;
        Err(())
    }

    pub fn init(&mut self) -> i32 {
        if self.sd_card_init() {
            0
        } else {
            -1
        }
    }
}

use spin::Mutex;

lazy_static! {
    pub static ref EMMC_CTL: Mutex<EmmcCtl> = Mutex::new(EmmcCtl::new());
}

fn demo() {
    // print out the first section of the sd_card.
    let section: [u8; 512] = [0; 512];
    let buf = unsafe { slice::from_raw_parts_mut(section.as_ptr() as *mut u32, 512 / 4) };
    println!("Trying to fetch the first section of the SD card.");
    if !EMMC_CTL.lock().read(0, 1, buf).is_ok() {
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

fn demo_write() {
    let section: [u8; 512] = [0; 512];
    let mut deadbeef: [u8; 512] = [0; 512];
    println!("Trying to fetch the second section of the SD card.");
    if !EMMC_CTL
        .lock()
        .read(1, 1, unsafe {
            slice::from_raw_parts_mut(section.as_ptr() as *mut u32, 512 / 4)
        })
        .is_ok()
    {
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

    if !EMMC_CTL
        .lock()
        .write(1, 1, unsafe {
            slice::from_raw_parts(deadbeef.as_ptr() as *mut u32, 512 / 4)
        })
        .is_ok()
    {
        error!("Failed in writing.");
        return;
    }
    if !EMMC_CTL
        .lock()
        .read(1, 1, unsafe {
            slice::from_raw_parts_mut(deadbeef.as_ptr() as *mut u32, 512 / 4)
        })
        .is_ok()
    {
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
    if !EMMC_CTL
        .lock()
        .write(1, 1, unsafe {
            slice::from_raw_parts(section.as_ptr() as *mut u32, 512 / 4)
        })
        .is_ok()
    {
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
    println!("Passed write() check.");
}

pub fn init() {
    debug!("Initializing EmmcCtl...");
    if EMMC_CTL.lock().init() == 0 {
        debug!("EmmcCtl successfully initialized.");
        //demo();
        //demo_write();
        info!("emmc: init end");
    } else {
        info!("emmc: init failed");
    }
}
