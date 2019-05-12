use bcm2837::emmc::*;
use core::time::Duration;
use crate::thread;

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
    (RESERVED, $index:expr) => (0xffff_ffff);
    (INDEX, $index:expr) => (($index) << 24);
}

const sd_commands: [u32; 64] = [
    sd_cmd!(INDEX,0),
    sd_cmd!(RESERVED,1),
    sd_cmd!(INDEX,2) | SD_RESP_R2,
    sd_cmd!(INDEX,3) | SD_RESP_R6,
    sd_cmd!(INDEX,4),
    sd_cmd!(INDEX,5) | SD_RESP_R4,
    sd_cmd!(INDEX,6) | SD_RESP_R1,
    sd_cmd!(INDEX,7) | SD_RESP_R1b,
    sd_cmd!(INDEX,8) | SD_RESP_R7,
    sd_cmd!(INDEX,9) | SD_RESP_R2,
    sd_cmd!(INDEX,10) | SD_RESP_R2,
    sd_cmd!(INDEX,11) | SD_RESP_R1,
    sd_cmd!(INDEX,12) | SD_RESP_R1b | SD_CMD_TYPE_ABORT,
    sd_cmd!(INDEX,13) | SD_RESP_R1,
    sd_cmd!(RESERVED,14),
    sd_cmd!(INDEX,15),
    sd_cmd!(INDEX,16) | SD_RESP_R1,
    sd_cmd!(INDEX,17) | SD_RESP_R1 | SD_DATA_READ,
    sd_cmd!(INDEX,18) | SD_RESP_R1 | SD_DATA_READ | SD_CMD_MULTI_BLOCK | SD_CMD_BLKCNT_EN,
    sd_cmd!(INDEX,19) | SD_RESP_R1 | SD_DATA_READ,
    sd_cmd!(INDEX,20) | SD_RESP_R1b,
    sd_cmd!(RESERVED,21),
    sd_cmd!(RESERVED,22),
    sd_cmd!(INDEX,23) | SD_RESP_R1,
    sd_cmd!(INDEX,24) | SD_RESP_R1 | SD_DATA_WRITE,
    sd_cmd!(INDEX,25) | SD_RESP_R1 | SD_DATA_WRITE | SD_CMD_MULTI_BLOCK | SD_CMD_BLKCNT_EN,
    sd_cmd!(RESERVED,26),
    sd_cmd!(INDEX,27) | SD_RESP_R1 | SD_DATA_WRITE,
    sd_cmd!(INDEX,28) | SD_RESP_R1b,
    sd_cmd!(INDEX,29) | SD_RESP_R1b,
    sd_cmd!(INDEX,30) | SD_RESP_R1 | SD_DATA_READ,
    sd_cmd!(RESERVED,31),
    sd_cmd!(INDEX,32) | SD_RESP_R1,
    sd_cmd!(INDEX,33) | SD_RESP_R1,
    sd_cmd!(RESERVED,34),
    sd_cmd!(RESERVED,35),
    sd_cmd!(RESERVED,36),
    sd_cmd!(RESERVED,37),
    sd_cmd!(INDEX,38) | SD_RESP_R1b,
    sd_cmd!(RESERVED,39),
    sd_cmd!(RESERVED,40),
    sd_cmd!(RESERVED,41),
    sd_cmd!(RESERVED,42) | SD_RESP_R1,
    sd_cmd!(RESERVED,43),
    sd_cmd!(RESERVED,44),
    sd_cmd!(RESERVED,45),
    sd_cmd!(RESERVED,46),
    sd_cmd!(RESERVED,47),
    sd_cmd!(RESERVED,48),
    sd_cmd!(RESERVED,49),
    sd_cmd!(RESERVED,50),
    sd_cmd!(RESERVED,51),
    sd_cmd!(RESERVED,52),
    sd_cmd!(RESERVED,53),
    sd_cmd!(RESERVED,54),
    sd_cmd!(INDEX,55) | SD_RESP_R1,
    sd_cmd!(INDEX,56) | SD_RESP_R1 | SD_CMD_ISDATA,
    sd_cmd!(RESERVED,57),
    sd_cmd!(RESERVED,58),
    sd_cmd!(RESERVED,59),
    sd_cmd!(RESERVED,60),
    sd_cmd!(RESERVED,61),
    sd_cmd!(RESERVED,62),
    sd_cmd!(RESERVED,63)
];

const sd_acommands: [u32; 64] = [
    sd_cmd!(RESERVED,0),
    sd_cmd!(RESERVED,1),
    sd_cmd!(RESERVED,2),
    sd_cmd!(RESERVED,3),
    sd_cmd!(RESERVED,4),
    sd_cmd!(RESERVED,5),
    sd_cmd!(INDEX,6) | SD_RESP_R1,
    sd_cmd!(RESERVED,7),
    sd_cmd!(RESERVED,8),
    sd_cmd!(RESERVED,9),
    sd_cmd!(RESERVED,10),
    sd_cmd!(RESERVED,11),
    sd_cmd!(RESERVED,12),
    sd_cmd!(INDEX,13) | SD_RESP_R1,
    sd_cmd!(RESERVED,14),
    sd_cmd!(RESERVED,15),
    sd_cmd!(RESERVED,16),
    sd_cmd!(RESERVED,17),
    sd_cmd!(RESERVED,18),
    sd_cmd!(RESERVED,19),
    sd_cmd!(RESERVED,20),
    sd_cmd!(RESERVED,21),
    sd_cmd!(INDEX,22) | SD_RESP_R1 | SD_DATA_READ,
    sd_cmd!(INDEX,23) | SD_RESP_R1,
    sd_cmd!(RESERVED,24),
    sd_cmd!(RESERVED,25),
    sd_cmd!(RESERVED,26),
    sd_cmd!(RESERVED,27),
    sd_cmd!(RESERVED,28),
    sd_cmd!(RESERVED,29),
    sd_cmd!(RESERVED,30),
    sd_cmd!(RESERVED,31),
    sd_cmd!(RESERVED,32),
    sd_cmd!(RESERVED,33),
    sd_cmd!(RESERVED,34),
    sd_cmd!(RESERVED,35),
    sd_cmd!(RESERVED,36),
    sd_cmd!(RESERVED,37),
    sd_cmd!(RESERVED,38),
    sd_cmd!(RESERVED,39),
    sd_cmd!(RESERVED,40),
    sd_cmd!(INDEX,41) | SD_RESP_R3,
    sd_cmd!(INDEX,42) | SD_RESP_R1,
    sd_cmd!(RESERVED,43),
    sd_cmd!(RESERVED,44),
    sd_cmd!(RESERVED,45),
    sd_cmd!(RESERVED,46),
    sd_cmd!(RESERVED,47),
    sd_cmd!(RESERVED,48),
    sd_cmd!(RESERVED,49),
    sd_cmd!(RESERVED,50),
    sd_cmd!(INDEX,51) | SD_RESP_R1 | SD_DATA_READ,
    sd_cmd!(RESERVED,52),
    sd_cmd!(RESERVED,53),
    sd_cmd!(RESERVED,54),
    sd_cmd!(RESERVED,55),
    sd_cmd!(RESERVED,56),
    sd_cmd!(RESERVED,57),
    sd_cmd!(RESERVED,58),
    sd_cmd!(RESERVED,59),
    sd_cmd!(RESERVED,60),
    sd_cmd!(RESERVED,61),
    sd_cmd!(RESERVED,62),
    sd_cmd!(RESERVED,63)
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

#[derive(Debug)]
pub struct SDScr {
    scr: [u32; 2],
    sd_bus_widths:  u32,
    sd_version: i32
}

impl SDScr {
    pub fn new() -> SDScr {
        SDScr {
            scr: [0u32; 2],
            sd_bus_widths: 0,
            sd_version: 0
        }
    }
}

pub struct EmmcCtl {
    emmc: Emmc,
    card_supports_sdhc: bool,
    card_supports_18v:  bool,
    card_ocr:           u32,
    card_rca:           u32,
    last_interrupt:     u32,
    last_error:         u32,

    sd_scr: SDScr,
    failed_voltage_switch: i32,

    last_cmd_reg:       u32,
    last_cmd:           u32,
    last_cmd_success:   bool,
    last_r:             [u32; 4],

    // void *buf;
    blocks_to_transfer: u32,
    block_size:         usize,
    use_sdma:           bool,
    card_removal:       bool,
    base_clock:         u32,
}

fn usleep(cnt: u32) {
    thread::sleep(Duration::from_micros(cnt.into()));
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
 * int sd_card_init(struct block_device **dev)
 * ++ int sd_read(struct block_device *dev, uint8_t *buf, size_t buf_size, uint32_t block_no)
 * ++ int sd_write(struct block_device *dev, uint8_t *buf, size_t buf_size, uint32_t block_no)
 * Other Constants
 */

const MAX_WAIT_US: u32 = 1000000;
const MAX_WAIT_TIMES: u32 = MAX_WAIT_US / 1000;

macro_rules! timeout_wait {
    ($condition:expr) => ({
        let mut succeeded = false;
        for _ in 0..MAX_WAIT_TIMES {
            if $condition {
                succeeded = true;
                break;
            }
            usleep(1000);
        }
        succeeded
    });

    ($condition:expr, $timeout:expr) => ({
        let mut succeeded = false;
        for _ in 0..(($timeout) / 1000) {
            if $condition {
                succeeded = true;
                break;
            }
            usleep(1000);
        }
        succeeded
    })
}

impl EmmcCtl {

    pub fn new() -> EmmcCtl { //TODO: improve it!
        EmmcCtl {
            emmc: Emmc::new(),
            card_supports_sdhc:false,
            card_supports_18v:false,
            card_ocr:0,
            card_rca:0,
            last_interrupt:0,
            last_error:0,

            sd_scr: SDScr::new(),
            failed_voltage_switch:0,

            last_cmd_reg:0,
            last_cmd:0,
            last_cmd_success:false,
            last_r:[0;4],

            blocks_to_transfer:0,
            block_size:0,
            use_sdma:false,
            card_removal:false,
            base_clock:0,
        }        
    }

    pub fn sd_power_off(&mut self) {
        let ctl0 = self.emmc.registers.CONTROL0.read();
        self.emmc.registers.CONTROL0.write(ctl0 & !(1 << 8));
    }

    pub fn sd_get_clock_divider(&mut self, base_clock: u32, target_rate: u32) -> u32 {
        let targetted_divisor: u32 = if (target_rate > base_clock) { 1 }
        else {
            base_clock / target_rate - if (base_clock % target_rate != 0) { 1 } else { 0 }
        };

        let mut divisor = 31;

        for first_bit in (0..32).rev() {
            if targetted_divisor & (1 << first_bit) != 0 {
                divisor = first_bit + if targetted_divisor != (1 << first_bit) { 1 } else { 0 };
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
        control1 &= !0xffe0;		// Clear old setting + clock generator select
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

        timeout_wait!(self.emmc.registers.CONTROL1.read() & SD_RESET_CMD == 0)
    }

    pub fn sd_reset_dat(&mut self) -> bool {
        let mut control1 = self.emmc.registers.CONTROL1.read();
        self.emmc.registers.CONTROL1.write(control1 | SD_RESET_DAT);

        timeout_wait!(self.emmc.registers.CONTROL1.read() & SD_RESET_DAT == 0)
    }

    pub fn sd_handle_card_interrupt(&mut self) {
        if self.card_rca != 0 {
            self.sd_issue_command_int(sd_commands[SEND_STATUS as usize], self.card_rca << 16, 500000);
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
        }

        if irpts & SD_BUFFER_READ_READY != 0 {
            reset_mask |= SD_BUFFER_READ_READY;
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

    pub fn sd_issue_command_int_pre(&mut self, command: u32, argument: u32, timeout: u32) -> bool{
        self.last_cmd_reg = command;
        self.last_cmd_success = false;

        while (self.emmc.registers.STATUS.read() & 0x1) != 0 {
            usleep(1000);
        }

        if ((command & SD_CMD_RSPNS_TYPE_MASK) == SD_CMD_RSPNS_TYPE_48B) 
            && ((command & SD_CMD_TYPE_MASK) == SD_CMD_TYPE_ABORT) {
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
        for i in 0..3 {
            self.last_r[i] = self.emmc.registers.RESP[i].read();
        }
        true
    }

    pub fn sd_issue_command_int_post(&mut self, command: u32, argument: u32, timeout: u32) -> bool{
        if ((command & SD_CMD_RSPNS_TYPE_MASK) == SD_CMD_RSPNS_TYPE_48B)
            || (command & SD_CMD_ISDATA) != 0 {
                if (self.emmc.registers.STATUS.read() & 0x2) == 0 {
                    self.emmc.registers.INTERRUPT.write(0xffff_0002);
                } else {
                    timeout_wait!((self.emmc.registers.INTERRUPT.read() & 0x8002) != 0, timeout);
                    let mut irpts = self.emmc.registers.INTERRUPT.read();
                    self.emmc.registers.INTERRUPT.write(0xffff_0002);

                    if (irpts & 0xffff_0002) != 0x2 && (irpts & 0xffff_0002) != 0x10_0002 {
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

    pub fn sd_issue_command(&mut self, command: u32, argument: u32, timeout: u32) -> bool{
        self.sd_handle_interrupts();
        if command & IS_APP_CMD != 0{
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
        }
        else {
            if sd_commands[command as usize] == sd_cmd!(RESERVED, 0) {
                self.last_cmd_success = false;
                return false;
            }
            self.last_cmd = command;
            self.sd_issue_command_int(sd_commands[command as usize], argument, timeout);
        }
        true
    }

    pub fn sd_check_success(&mut self) -> bool {
        if self.last_cmd_success {
            return true;
        }
        else {
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

    pub fn read(&mut self, block_no_arg: u32, count: usize, buf: &mut[u32]) ->  Result<(), ()> {
        let mut block_no = block_no_arg;
        if self.card_supports_sdhc {
            block_no *= 512;
        }
        //assert_eq!(count * self.block_size, buf.len());
        self.blocks_to_transfer = count as u32;
        let mut command = 0;
        if count > 1 {
            command = READ_MULTIPLE_BLOCK;
        } else {
            command = READ_SINGLE_BLOCK;
        }
        for retry in 0..3 {
            { // send command
                self.last_cmd = command;
                if self.sd_issue_command_int_pre(command, block_no, 500000) {
                    let mut wr_irpt = (1<<5);
                    let mut finished = true;
                    for cur_block in 0..count {
                        timeout_wait!(self.emmc.registers.INTERRUPT.read() & (wr_irpt | 0x8000) != 0, 500000);
                        let irpts = self.emmc.registers.INTERRUPT.read();
                        self.emmc.registers.INTERRUPT.write(0xffff_0000 | wr_irpt);
                        if (irpts & (0xffff_0000 | wr_irpt)) != wr_irpt {
                            self.last_error = irpts & 0xffff_0000;
                            self.last_interrupt = irpts;
                            finished = false;
                            break;
                        }
                        let mut cur_byte_no = 0;
                        while (cur_byte_no < self.block_size) {
                            buf[(cur_block as usize)* self.block_size + cur_byte_no] = 
                                self.emmc.registers.DATA.read();
                            cur_byte_no += 4;
                        }
                    }
                    if finished {
                        self.sd_issue_command_int_post(command, block_no, 500000);
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

    pub fn write(&mut self, block_no_arg: u32, count: usize, buf: &[u32]) ->  Result<(), ()> {
        let mut block_no = block_no_arg;
        if self.card_supports_sdhc {
            block_no *= 512;
        }
        //assert_eq!(count * self.block_size, buf.len());
        self.blocks_to_transfer = count as u32;
        let mut command = 0;
        if count > 1 {
            command = READ_MULTIPLE_BLOCK;
        } else {
            command = READ_SINGLE_BLOCK;
        }
        for retry in 0..3 {
            { // send command
                self.last_cmd = command;
                if self.sd_issue_command_int_pre(command, block_no, 500000) {
                    let mut wr_irpt = (1<<4);
                    let mut finished = true;
                    for cur_block in 0..count {
                        timeout_wait!(self.emmc.registers.INTERRUPT.read() & (wr_irpt | 0x8000) != 0, 500000);
                        let irpts = self.emmc.registers.INTERRUPT.read();
                        self.emmc.registers.INTERRUPT.write(0xffff_0000 | wr_irpt);
                        if (irpts & (0xffff_0000 | wr_irpt)) != wr_irpt {
                            self.last_error = irpts & 0xffff_0000;
                            self.last_interrupt = irpts;
                            finished = false;
                            break;
                        }
                        let mut cur_byte_no = 0;
                        while (cur_byte_no < self.block_size) {
                            self.emmc.registers.DATA.write(buf[(cur_block as usize)* self.block_size + cur_byte_no]);
                            cur_byte_no += 4;
                        }
                    }
                    if finished {
                        self.sd_issue_command_int_post(command, block_no, 500000);
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
        0
    }
}