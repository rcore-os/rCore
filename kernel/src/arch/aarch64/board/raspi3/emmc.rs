use bcm2837::emmc::*;
use core::time::Duration;
use crate::thread;

struct EmmcCtl {
    emmc: Emmc,
}

fn usleep(cnt: u32) {
    thread::sleep(Duration::from_micros(cnt.into()));
}

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