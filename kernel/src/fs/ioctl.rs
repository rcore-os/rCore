// for IOR and IOW:
// 32bits total, command in lower 16bits, size of the parameter structure in the lower 14 bits of the upper 16 bits
// higher 2 bits: 01 = write, 10 = read
#![allow(dead_code)]

use bitflags::*;

#[cfg(not(target_arch = "mips"))]
pub const TCGETS: usize = 0x5401;
#[cfg(target_arch = "mips")]
pub const TCGETS: usize = 0x540D;

#[cfg(not(target_arch = "mips"))]
pub const TCSETS: usize = 0x5402;
#[cfg(target_arch = "mips")]
pub const TCGETS: usize = 0x540E;

#[cfg(not(target_arch = "mips"))]
pub const TIOCGPGRP: usize = 0x540F;
// _IOR('t', 119, int)
#[cfg(target_arch = "mips")]
pub const TIOCGPGRP: usize = 0x4_004_74_77;

#[cfg(not(target_arch = "mips"))]
pub const TIOCSPGRP: usize = 0x5410;
// _IOW('t', 118, int)
#[cfg(target_arch = "mips")]
pub const TIOCSPGRP: usize = 0x8_004_74_76;

#[cfg(not(target_arch = "mips"))]
pub const TIOCGWINSZ: usize = 0x5413;
// _IOR('t', 104, struct winsize)
#[cfg(target_arch = "mips")]
pub const TIOCGWINSZ: usize = 0x4_008_74_68;

#[cfg(not(target_arch = "mips"))]
pub const FIONCLEX: usize = 0x5450;
#[cfg(target_arch = "mips")]
pub const FIONCLEX: usize = 0x6602;

#[cfg(not(target_arch = "mips"))]
pub const FIOCLEX: usize = 0x5451;
#[cfg(target_arch = "mips")]
pub const FIOCLEX: usize = 0x6601;

// rustc using pipe and ioctl pipe file with this request id
// for non-blocking/blocking IO control setting
pub const FIONBIO: usize = 0x5421;

bitflags! {
    pub struct LocalModes : u32 {
        const ISIG = 0o000001;
        const ICANON = 0o000002;
        const ECHO = 0o000010;
        const ECHOE = 0o000020;
        const ECHOK = 0o000040;
        const ECHONL = 0o000100;
        const NOFLSH = 0o000200;
        const TOSTOP = 0o000400;
        const IEXTEN = 0o100000;
        const XCASE = 0o000004;
        const ECHOCTL = 0o001000;
        const ECHOPRT = 0o002000;
        const ECHOKE = 0o004000;
        const FLUSHO = 0o010000;
        const PENDIN = 0o040000;
        const EXTPROC = 0o200000;
    }
}

// Ref: https://www.man7.org/linux/man-pages/man3/termios.3.html
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Termios {
    pub iflag: u32,
    pub oflag: u32,
    pub cflag: u32,
    pub lflag: u32,
    pub line: u8,
    pub cc: [u8; 32],
    pub ispeed: u32,
    pub ospeed: u32,
}

impl Default for Termios {
    fn default() -> Self {
        Termios {
            // IMAXBEL | IUTF8 | IXON | IXANY | ICRNL | BRKINT
            iflag: 0o66402,
            // OPOST | ONLCR
            oflag: 0o5,
            // HUPCL | CREAD | CSIZE | EXTB
            cflag: 0o2277,
            // IEXTEN | ECHOTCL | ECHOKE ECHO | ECHOE | ECHOK | ISIG | ICANON
            lflag: 0o105073,
            line: 0,
            cc: [
                3,   // VINTR Ctrl-C
                28,  // VQUIT
                127, // VERASE
                21,  // VKILL
                4,   // VEOF Ctrl-D
                0,   // VTIME
                1,   // VMIN
                0,   // VSWTC
                17,  // VSTART
                19,  // VSTOP
                26,  // VSUSP Ctrl-Z
                255, // VEOL
                18,  // VREPAINT
                15,  // VDISCARD
                23,  // VWERASE
                22,  // VLNEXT
                255, // VEOL2
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            ispeed: 0,
            ospeed: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Winsize {
    row: u16,
    ws_col: u16,
    xpixel: u16,
    ypixel: u16,
}
