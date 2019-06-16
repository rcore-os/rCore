///AArch64 Relocation Constants.
pub const R_MIPS_NONE: usize = 0;
pub const R_MIPS_32: usize = 2;
pub const R_MIPS_REL32: usize = 3;
pub const R_MIPS_GLOB_DAT: usize = 51;
pub const R_MIPS_JUMP_SLOT: usize = 127;
pub const R_MIPS_COPY: usize = 126;
pub const R_MIPS_TLS_DTPMOD32: usize = 38;
pub const R_MIPS_TLS_DTPREL32: usize = 39;
pub const R_MIPS_TLS_TPREL32: usize = 47;

pub const REL_NONE: usize = R_MIPS_NONE;
pub const REL_SYMBOLIC: usize = R_MIPS_32;
pub const REL_OFFSET32: usize = R_MIPS_REL32;
pub const REL_GOT: usize = R_MIPS_GLOB_DAT;
pub const REL_PLT: usize = R_MIPS_JUMP_SLOT;
pub const REL_RELATIVE: usize = 0; // dunno
pub const REL_COPY: usize = R_MIPS_COPY;
pub const REL_DTPMOD: usize = R_MIPS_TLS_DTPMOD32;
pub const REL_DTPOFF: usize = R_MIPS_TLS_DTPREL32;
pub const REL_TPOFF: usize = R_MIPS_TLS_TPREL32;
pub const REL_TLSDESC: usize = 0; // dunno
