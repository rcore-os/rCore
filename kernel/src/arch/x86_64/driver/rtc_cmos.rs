//! Driver for x86 CMOS RTC clock
use crate::arch::interrupt;
use x86_64::instructions::port::Port;

const CMOS_ADDR: u16 = 0x70;
const CMOS_DATA: u16 = 0x71;

unsafe fn check_updating() -> bool {
    let mut addr = Port::<u8>::new(CMOS_ADDR);
    let mut data = Port::<u8>::new(CMOS_DATA);

    addr.write(0x0A);
    return (data.read() & 0x80) != 0;
}

unsafe fn read_rtc(reg: u8) -> u8 {
    let mut addr = Port::<u8>::new(CMOS_ADDR);
    let mut data = Port::<u8>::new(CMOS_DATA);

    addr.write(reg);
    return data.read();
}

fn bcd2bin(num: u64) -> u64 {
    (num & 0x0f) + (num >> 4) * 10
}

// read seconds since 1970-01-01
pub fn read_epoch() -> u64 {
    unsafe {
        let flags = interrupt::disable_and_store();

        while check_updating() {}

        let mut second = read_rtc(0x00) as u64;
        let mut minute = read_rtc(0x02) as u64;
        let mut hour = read_rtc(0x04) as u64;
        let mut day = read_rtc(0x07) as u64;
        let mut month = read_rtc(0x08) as u64;
        let mut year = read_rtc(0x09) as u64;

        let control = read_rtc(0x0B);
        if (control & 0x04) == 0 {
            // BCD
            second = bcd2bin(second);
            minute = bcd2bin(minute);
            hour = bcd2bin(hour);
            day = bcd2bin(day);
            month = bcd2bin(month);
            year = bcd2bin(year);
        }

        // TODO: parse ACPI and find century register
        year += 2000;

        // mktime64
        if month <= 2 {
            month = month + 10;
            year = year - 1;
        } else {
            month = month - 2;
        }

        let result = ((((year / 4 - year / 100 + year / 400 + 367 * month / 12 + day)
            + year * 365
            - 719499)
            * 24
            + hour)
            * 60
            + minute)
            * 60
            + second;

        interrupt::restore(flags);

        result
    }
}
