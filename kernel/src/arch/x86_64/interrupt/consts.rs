#![allow(non_upper_case_globals)]
// Reference: https://wiki.osdev.org/Exceptions

pub const DivideError: u8 = 0;
pub const Debug: u8 = 1;
pub const NonMaskableInterrupt: u8 = 2;
pub const Breakpoint: u8 = 3;
pub const Overflow: u8 = 4;
pub const BoundRangeExceeded: u8 = 5;
pub const InvalidOpcode: u8 = 6;
pub const DeviceNotAvailable: u8 = 7;
pub const DoubleFault: u8 = 8;
pub const CoprocessorSegmentOverrun: u8 = 9;
pub const InvalidTSS: u8 = 10;
pub const SegmentNotPresent: u8 = 11;
pub const StackSegmentFault: u8 = 12;
pub const GeneralProtectionFault: u8 = 13;
pub const PageFault: u8 = 14;
pub const FloatingPointException: u8 = 16;
pub const AlignmentCheck: u8 = 17;
pub const MachineCheck: u8 = 18;
pub const SIMDFloatingPointException: u8 = 19;
pub const VirtualizationException: u8 = 20;
pub const SecurityException: u8 = 30;

pub const IRQ0: u8 = 32;
pub const Syscall32: u8 = 0x80;

// IRQ
pub const Timer: u8 = 0;
pub const Keyboard: u8 = 1;
pub const COM2: u8 = 3;
pub const COM1: u8 = 4;
pub const IDE: u8 = 14;
pub const Error: u8 = 19;
pub const Spurious: u8 = 31;

// PCI Interrupts
// See https://gist.github.com/mcastelino/4acda7c2407f1c51e68f3f994d8ffc98
pub const PIRQA: u8 = 16;
pub const PIRQB: u8 = 17;
pub const PIRQC: u8 = 18;
pub const PIRQD: u8 = 19;
pub const PIRQE: u8 = 20;
pub const PIRQF: u8 = 21;
pub const PIRQG: u8 = 22;
pub const PIRQH: u8 = 23;

// IPI constants
pub const IPIFuncCall: u8 = 0xfc;
