#![allow(non_upper_case_globals)]
// Reference: https://wiki.osdev.org/Exceptions

pub const DivideError: usize = 0;
pub const Debug: usize = 1;
pub const NonMaskableInterrupt: usize = 2;
pub const Breakpoint: usize = 3;
pub const Overflow: usize = 4;
pub const BoundRangeExceeded: usize = 5;
pub const InvalidOpcode: usize = 6;
pub const DeviceNotAvailable: usize = 7;
pub const DoubleFault: usize = 8;
pub const CoprocessorSegmentOverrun: usize = 9;
pub const InvalidTSS: usize = 10;
pub const SegmentNotPresent: usize = 11;
pub const StackSegmentFault: usize = 12;
pub const GeneralProtectionFault: usize = 13;
pub const PageFault: usize = 14;
pub const FloatingPointException: usize = 16;
pub const AlignmentCheck: usize = 17;
pub const MachineCheck: usize = 18;
pub const SIMDFloatingPointException: usize = 19;
pub const VirtualizationException: usize = 20;
pub const SecurityException: usize = 30;

pub const IRQ0: usize = 32;
pub const Syscall32: usize = 0x80;

// IRQ
pub const Timer: usize = 0;
pub const Keyboard: usize = 1;
pub const COM2: usize = 3;
pub const COM1: usize = 4;
pub const Error: usize = 19;
pub const Spurious: usize = 31;

// PCI Interrupts
// See https://gist.github.com/mcastelino/4acda7c2407f1c51e68f3f994d8ffc98
pub const PIRQA: usize = 16;
pub const PIRQB: usize = 17;
pub const PIRQC: usize = 18;
pub const PIRQD: usize = 19;
pub const PIRQE: usize = 20;
pub const PIRQF: usize = 21;
pub const PIRQG: usize = 22;
pub const PIRQH: usize = 23;

// IPI constants
pub const IPIFuncCall: usize = 0xfc;
