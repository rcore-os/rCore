pub mod driver;
pub mod cpu;
pub mod interrupt;

pub fn init() {
	cpu::enable_nxe_bit();
	cpu::enable_write_protect_bit();
}