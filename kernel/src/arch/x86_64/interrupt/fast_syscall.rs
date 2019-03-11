/// `syscall` instruction

use x86_64::registers::model_specific::*;
use core::mem::transmute;
use super::super::gdt;
use super::TrapFrame;

pub fn init() {
    unsafe {
        Efer::update(|flags| {
            *flags |= EferFlags::SYSTEM_CALL_EXTENSIONS;
        });

        let mut star = Msr::new(0xC0000081);
        let mut lstar = Msr::new(0xC0000082);
        let mut sfmask = Msr::new(0xC0000084);

        // flags to clear on syscall
        // copy from Linux 5.0
        // TF|DF|IF|IOPL|AC|NT
        let rflags_mask = 0x47700;

        star.write(transmute(STAR));
        lstar.write(syscall_entry as u64);
        sfmask.write(rflags_mask);
    }
}

extern {
    fn syscall_entry();
}

#[repr(packed)]
struct StarMsr {
    eip: u32,
    kernel_cs: u16,
    user_cs: u16,
}

const STAR: StarMsr = StarMsr {
    eip: 0, // ignored in 64 bit mode
    kernel_cs: gdt::KCODE_SELECTOR.0,
    user_cs: gdt::UCODE32_SELECTOR.0,
};
