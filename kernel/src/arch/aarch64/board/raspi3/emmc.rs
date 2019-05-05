use bcm2837::emmc::*;
use core::time::Duration;
use crate::thread;

struct EmmcCtl {
    emmc: Emmc,
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
 * static int sd_suitable_for_dma(void *buf)
 * static int sd_do_data_command(struct emmc_block_dev *edev, int is_write, uint8_t *buf, size_t buf_size, uint32_t block_no)
 * Other Constants
 */

impl EmmcCtl {

    pub fn new() -> EmmcCtl {
        EmmcCtl {
            emmc: Emmc::new(),
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
}