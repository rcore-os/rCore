pub fn read_epoch() -> u64 {
    super::driver::rtc_cmos::read_epoch()
}
