pub mod driver;
pub mod cpu;

pub fn init() {
	cpu::enable_nxe_bit();
	cpu::enable_write_protect_bit();
}