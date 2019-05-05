use bcm2837::emmc::*;
use core::time::Duration;
use crate::thread;

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
    card_supports_sdhc: u32,
    card_supports_18v:  u32,
    card_ocr:           u32,
    card_rca:           u32,
    last_interrupt:     u32,
    last_error:         u32,

    sd_scr: SDScr,
    failed_voltage_switch: i32,

    last_cmd_reg:       u32,
    last_cmd:           u32,
    last_cmd_success:   u32,
    last_r0:            u32,
    last_r1:            u32,
    last_r2:            u32,
    last_r3:            u32,

    // void *buf;
    // int blocks_to_transfer;
    block_size:         usize,
    use_sdma:           i32,
    card_removal:       i32,
    base_clock:         u32
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
 * static int sd_reset_cmd()
 * static int sd_reset_dat()
 * static void sd_issue_command_int(struct emmc_block_dev *dev, uint32_t cmd_reg, uint32_t argument, useconds_t timeout)
 * static void sd_handle_card_interrupt(struct emmc_block_dev *dev)
 * static void sd_handle_interrupts(struct emmc_block_dev *dev)
 * 
 * 
 * static void sd_issue_command(struct emmc_block_dev *dev, uint32_t command, uint32_t argument, useconds_t timeout)
 * static int sd_ensure_data_mode(struct emmc_block_dev *edev)
 * -- static int sd_suitable_for_dma(void *buf)
 * static int sd_do_data_command(struct emmc_block_dev *edev, int is_write, uint8_t *buf, size_t buf_size, uint32_t block_no)
 * int sd_card_init(struct block_device **dev)
 * int sd_read(struct block_device *dev, uint8_t *buf, size_t buf_size, uint32_t block_no)
 * int sd_write(struct block_device *dev, uint8_t *buf, size_t buf_size, uint32_t block_no)
 * Other Constants
 */

const MAX_WAIT_US: u32 = 1000000;
const MAX_WAIT_TIMES: u32 = MAX_WAIT_US / 1000;

impl EmmcCtl {

    pub fn new() -> EmmcCtl { //TODO: improve it!
        EmmcCtl {
            emmc: Emmc::new(),
            card_supports_sdhc:0,
            card_supports_18v:0,
            card_ocr:0,
            card_rca:0,
            last_interrupt:0,
            last_error:0,

            sd_scr: SDScr::new(),
            failed_voltage_switch:0,

            last_cmd_reg:0,
            last_cmd:0,
            last_cmd_success:0,
            last_r0:0,
            last_r1:0,
            last_r2:0,
            last_r3:0,

            block_size:0,
            use_sdma:0,
            card_removal:0,
            base_clock:0
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

        let mut succeeded = false;
        for _ in 0..MAX_WAIT_TIMES {
            if self.emmc.registers.CONTROL1.read() & SD_RESET_CMD == 0 {
                succeeded = true;
                break;
            }
        }

        succeeded
    }

    pub fn sd_reset_dat(&mut self) -> bool {
        let mut control1 = self.emmc.registers.CONTROL1.read();
        self.emmc.registers.CONTROL1.write(control1 | SD_RESET_DAT);

        let mut succeeded = false;
        for _ in 0..MAX_WAIT_TIMES {
            if self.emmc.registers.CONTROL1.read() & SD_RESET_DAT == 0 {
                succeeded = true;
                break;
            }
        }

        succeeded
    }

    pub fn sd_issue_command_int(&mut self, cmd_reg: u32, argument: u32, timeout: u32) {

    }

    pub fn sd_handle_card_interrupt(&mut self) {

    }

    pub fn sd_handle_interrupts(&mut self) {

    }

    pub fn sd_issue_command(&mut self, command: u32, argument: u32, timeout: u32) {

    }

    pub fn sd_ensure_data_mode(&mut self) -> i32 {
        0
    }

    // sdma not implemented.
    // pub fn sd_suitable_for_dma()

    pub fn sd_do_data_command(&mut self, is_write: bool, buf: &[u8], block_no: u32) -> i32 {
        0
    }

    pub fn read(&mut self, buf: &[u8], block_no: u32) -> i32 {
        0
    }

    pub fn write(&mut self, buf: &[u8], block_no: u32) -> i32 {
        0
    }

    pub fn init(&mut self) -> i32 {
        0
    }
}