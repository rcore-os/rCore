pub const MAX_CPU_NUM: usize = 8;

pub mod irq {
	pub const T_IRQ0       : u8 = 32;      // IRQ 0 corresponds to int T_IRQ
	pub const IRQ_TIMER    : u8 =  0;
	pub const IRQ_KBD      : u8 =  1;
	pub const IRQ_COM1     : u8 =  4;
	pub const IRQ_IDE      : u8 = 14;
	pub const IRQ_ERROR    : u8 = 19;
	pub const IRQ_SPURIOUS : u8 = 31;
}
