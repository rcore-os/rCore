/// x86_64 Relocation Constants.
pub const R_X86_64_NONE: usize = 0;
pub const R_X86_64_64: usize = 1;
pub const R_X86_64_PC32: usize = 2;
pub const R_X86_64_GOT32: usize = 3;
pub const R_X86_64_PLT32: usize = 4;
pub const R_X86_64_COPY: usize = 5;
pub const R_X86_64_GLOB_DAT: usize = 6;
pub const R_X86_64_JUMP_SLOT: usize = 7;
pub const R_X86_64_RELATIVE: usize = 8;
pub const R_X86_64_GOTPCREL: usize = 9;

pub const R_X86_64_32: usize = 10;
pub const R_X86_64_32S: usize = 11;
pub const R_X86_64_16: usize = 12;
pub const R_X86_64_PC16: usize = 13;
pub const R_X86_64_8: usize = 14;
pub const R_X86_64_PC8: usize = 15;
pub const R_X86_64_DTPMOD64: usize = 16;
pub const R_X86_64_DTPOFF64: usize = 17;
pub const R_X86_64_TPOFF64: usize = 18;
pub const R_X86_64_TLSGD: usize = 19;

pub const R_X86_64_TLSLD: usize = 20;

pub const R_X86_64_DTPOFF32: usize = 21;
pub const R_X86_64_GOTTPOFF: usize = 22;

pub const R_X86_64_TPOFF32: usize = 23;
pub const R_X86_64_PC64: usize = 24;
pub const R_X86_64_GOTOFF64: usize = 25;
pub const R_X86_64_GOTPC32: usize = 26;
pub const R_X86_64_GOT64: usize = 27;
pub const R_X86_64_GOTPCREL64: usize = 28;
pub const R_X86_64_GOTPC64: usize = 29;
pub const R_X86_64_GOTPLT64: usize = 30;
pub const R_X86_64_PLTOFF64: usize = 31;
pub const R_X86_64_SIZE32: usize = 32;
pub const R_X86_64_SIZE64: usize = 33;

pub const R_X86_64_GOTPC32_TLSDESC: usize = 34;
pub const R_X86_64_TLSDESC_CALL: usize = 35;
pub const R_X86_64_TLSDESC: usize = 36;
pub const R_X86_64_IRELATIVE: usize = 37;
pub const R_X86_64_RELATIVE64: usize = 38;
pub const R_X86_64_GOTPCRELX: usize = 41;
pub const R_X86_64_REX_GOTPCRELX: usize = 42;
pub const R_X86_64_NUM: usize = 43;

pub const REL_NONE: usize = R_X86_64_NONE;
pub const REL_SYMBOLIC: usize = R_X86_64_64;
pub const REL_OFFSET32: usize = R_X86_64_PC32;
pub const REL_GOT: usize = R_X86_64_GLOB_DAT;
pub const REL_PLT: usize = R_X86_64_JUMP_SLOT;
pub const REL_RELATIVE: usize = R_X86_64_RELATIVE;
pub const REL_COPY: usize = R_X86_64_COPY;
pub const REL_DTPMOD: usize = R_X86_64_DTPMOD64;
pub const REL_DTPOFF: usize = R_X86_64_DTPOFF64;
pub const REL_TPOFF: usize = R_X86_64_TPOFF64;
pub const REL_TLSDESC: usize = R_X86_64_TLSDESC;
