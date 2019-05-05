use bcm2837::emmc::*;
use core::time::Duration;
use crate::thread;

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
 * static void sd_issue_command(struct emmc_block_dev *dev, uint32_t command, uint32_t argument, useconds_t timeout)
 * static int sd_ensure_data_mode(struct emmc_block_dev *edev)
 * -- static int sd_suitable_for_dma(void *buf)
 * static int sd_do_data_command(struct emmc_block_dev *edev, int is_write, uint8_t *buf, size_t buf_size, uint32_t block_no)
 * int sd_card_init(struct block_device **dev)
 * int sd_read(struct block_device *dev, uint8_t *buf, size_t buf_size, uint32_t block_no)
 * int sd_write(struct block_device *dev, uint8_t *buf, size_t buf_size, uint32_t block_no)
 * Other Constants
 */

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
        false
    }

    pub fn sd_reset_dat(&mut self) -> bool {
        false
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