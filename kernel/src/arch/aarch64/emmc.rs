use bcm2837::emmc::*;

struct EmmcCtl {
    emmc: Emmc,
}

impl EmmcCtl {
    pub fn new() -> EmmcCtl {
        EmmcCtl {
            emmc: Emmc::new(),
        }
    }
}