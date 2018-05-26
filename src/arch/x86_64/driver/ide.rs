//! ATA IO code, handling device multiplexing and IO operations
//!
//! Borrow from Rucore project. Thanks GWord!
//! Port from ucore C code.
use x86_64::instructions::port;
use spin::Mutex;

lazy_static! {
    pub static ref DISK0: LockedIde = LockedIde(Mutex::new(DmaController::new(0)));
}
pub const BLOCK_SIZE: usize = 512;

pub struct LockedIde(pub Mutex<DmaController>);

pub struct DmaController {
    num: u8,
}

impl DmaController
{
    /// Read ATA DMA. Block size = 512 bytes.
    pub fn read(&self, blockidx: u64, count: usize, dst: &mut [u32]) -> Result<usize, ()> {
        assert_eq!(dst.len(), count * SECTOR_SIZE);
        let dst = if count > MAX_DMA_SECTORS { &mut dst[..MAX_DMA_SECTORS * SECTOR_SIZE] } else { dst };
        //self.do_dma(blockidx, DMABuffer::new_mut(dst, 32), disk, false);
        self.ide_read_secs(self.num, blockidx, dst, count as u8)
    }
    /// Write ATA DMA. Block size = 512 bytes.
    pub fn write(&self, blockidx: u64, count: usize, dst: &[u32]) -> Result<usize, ()> {
        assert_eq!(dst.len(), count * SECTOR_SIZE);
        let dst = if count > MAX_DMA_SECTORS { &dst[..MAX_DMA_SECTORS * SECTOR_SIZE] } else { dst };
        //println!("ide_write_secs: disk={},blockidx={},count={}",disk,blockidx,count);
        self.ide_write_secs(self.num, blockidx, dst, count as u8)
    }
    /// Create structure and init
    fn new(num: u8) -> Self {
        assert!(num < MAX_IDE as u8);
        let ide = DmaController { num };
        ide.ide_init();
        ide
    }

    fn ide_wait_ready(&self, iobase: u16, check_error: usize) -> usize {
        unsafe {
            let mut r = port::inb(iobase + ISA_STATUS);
            //println!("iobase:{} ready:{}",iobase,r);
            while (r & IDE_BSY) > 0 {
                r = port::inb(iobase + ISA_STATUS);
                //println!("busy");
            }
            /* nothing */
            if check_error == 1 && (r & (IDE_DF | IDE_ERR)) != 0 {
                return 1;
            }
        }
        return 0;
    }

    fn ide_init(&self) {
        //static_assert((SECTSIZE % 4) == 0);
        let ideno = self.num;
        //println!("ideno:{}",ideno);
        /* assume that no device here */
        //ide_devices[ideno].valid = 0;

        //let iobase = IO_BASE(ideno);
        let iobase = CHANNELS[if ideno > 2 { 1 } else { 0 }].0;

        /* wait device ready */
        self.ide_wait_ready(iobase, 0);
        //println!("ide_wait_ready");
        unsafe {
            /* step1: select drive */
            //println!("outb");
            port::outb(iobase + ISA_SDH, (0xE0 | ((ideno & 1) << 4)) as u8);
            self.ide_wait_ready(iobase, 0);

            /* step2: send ATA identify command */
            //println!("outb");
            port::outb(iobase + ISA_COMMAND, IDE_CMD_IDENTIFY);
            self.ide_wait_ready(iobase, 0);

            /* step3: polling */
            //println!("inb");
            if port::inb(iobase + ISA_STATUS) == 0 || self.ide_wait_ready(iobase, 1) != 0 {
                return;
            }

            //println!("insl");
            let mut buffer: [u32; 128] = [0; 128];
            for i in 0..buffer.len() {
                buffer[i] = i as u32;
                if i == 1 {
                    //println!("{:#x}",&buffer[i] as *const u32 as usize - ::consts::KERNEL_OFFSET)
                }
            }
            //println!("insl {:#x}",&buffer as *const u32 as usize - ::consts::KERNEL_OFFSET);

            //println!("insl {:#x}",buffer.as_ptr() as usize - ::consts::KERNEL_OFFSET);
            //port::insl(iobase + ISA_DATA, &mut buffer);
            let port = iobase + ISA_DATA;
            //let buf=&mut buffer;
            for i in 0..buffer.len() {
                asm!("insl %dx, (%rdi)"
                :: "{dx}"(port), "{rdi}"(&buffer[i])
                : "rdi" : "volatile");
            }
            //println!("insl");
            for i in 0..4 {
                info!("ide init: {}", buffer[i]);
            }
        }
        /* device is ok */
        //ide_devices[ideno].valid = 1;

        /* read identification space of the device */
        /*let buffer[128];
        insl(iobase + ISA_DATA, buffer, sizeof(buffer) / sizeof(unsigned int));

        unsigned char *ident = (unsigned char *)buffer;
        unsigned int sectors;
        unsigned int cmdsets = *(unsigned int *)(ident + IDE_IDENT_CMDSETS);
        /* device use 48-bits or 28-bits addressing */
        if (cmdsets & (1 << 26)) {
            sectors = *(unsigned int *)(ident + IDE_IDENT_MAX_LBA_EXT);
        }
        else {
            sectors = *(unsigned int *)(ident + IDE_IDENT_MAX_LBA);
        }
        ide_devices[ideno].sets = cmdsets;
        ide_devices[ideno].size = sectors;

        /* check if supports LBA */
        assert((*(unsigned short *)(ident + IDE_IDENT_CAPABILITIES) & 0x200) != 0);

        unsigned char *model = ide_devices[ideno].model, *data = ident + IDE_IDENT_MODEL;
        unsigned int i, length = 40;
        for (i = 0; i < length; i += 2) {
            model[i] = data[i + 1], model[i + 1] = data[i];
        }
        do {
            model[i] = '\0';
        } while (i -- > 0 && model[i] == ' ');

        cprintf("ide %d: %10u(sectors), '%s'.\n", ideno, ide_devices[ideno].size, ide_devices[ideno].model);*/

        // enable ide interrupt
        //pic_enable(IRQ_IDE1);
        //pic_enable(IRQ_IDE2);

        info!("ide {} init end", self.num);
    }
    fn ide_read_secs<'a>(&'a self, ideno: u8, secno: u64, dst: &'a mut [u32], nsecs: u8) -> Result<usize, ()> {
        //assert(nsecs <= MAX_NSECS && VALID_IDE(ideno));
        //assert(secno < MAX_DISK_NSECS && secno + nsecs <= MAX_DISK_NSECS);
        let iobase = CHANNELS[if ideno > 2 { 1 } else { 0 }].0;
        let ioctrl = CHANNELS[if ideno > 2 { 1 } else { 0 }].1;

        //ide_wait_ready(iobase, 0);

        self.ide_wait_ready(iobase, 0);

        let ret = 0;
        // generate interrupt
        unsafe {
            port::outb(ioctrl + ISA_CTRL, 0);
            port::outb(iobase + ISA_SECCNT, nsecs);
            port::outb(iobase + ISA_SECTOR, (secno & 0xFF) as u8);
            port::outb(iobase + ISA_CYL_LO, ((secno >> 8) & 0xFF) as u8);
            port::outb(iobase + ISA_CYL_HI, ((secno >> 16) & 0xFF) as u8);
            port::outb(iobase + ISA_SDH, 0xE0 | ((ideno & 1) << 4) | (((secno >> 24) & 0xF) as u8));
            //port::outb(iobase + ISA_SDH, (0xE0 | ((ideno & 1) << 4)) as u8);
            //self.ide_wait_ready(iobase, 0);
            port::outb(iobase + ISA_COMMAND, IDE_CMD_READ);
            //self.ide_wait_ready(iobase, 0);
            // if port::inb(iobase + ISA_STATUS) == 0 || self.ide_wait_ready(iobase, 1) != 0 {
            // 	println!("error?");
            // }
            for i in 0..nsecs {
                //dst = dst + SECTSIZE;
                let tmp = &mut dst[(i as usize) * SECTOR_SIZE..((i + 1) as usize) * SECTOR_SIZE];
                if self.ide_wait_ready(iobase, 1) != 0 {
                    println!("wait ready error");
                }
                //self.ide_wait_ready(iobase, 1);
                //port::insl(iobase, tmp);
                let port = iobase;
                //let buf=&mut buffer;
                for i in 0..tmp.len() {
                    asm!("insl %dx, (%rdi)"
					:: "{dx}"(port), "{rdi}"(&tmp[i])
					: "rdi" : "volatile");
                }
                //println!("read :{}",i);
            }
        }
        Ok(ret)
    }

    fn ide_write_secs<'a>(&'a self, ideno: u8, secno: u64, src: &'a [u32], nsecs: u8) -> Result<usize, ()> {
        //assert(nsecs <= MAX_NSECS && VALID_IDE(ideno));
        //assert(secno < MAX_DISK_NSECS && secno + nsecs <= MAX_DISK_NSECS);
        let iobase = CHANNELS[if ideno > 2 { 1 } else { 0 }].0;
        let ioctrl = CHANNELS[if ideno > 2 { 1 } else { 0 }].1;

        //ide_wait_ready(iobase, 0);

        self.ide_wait_ready(iobase, 0);

        let ret = 0;
        // generate interrupt
        unsafe {
            port::outb(ioctrl + ISA_CTRL, 0);
            port::outb(iobase + ISA_SECCNT, nsecs);
            port::outb(iobase + ISA_SECTOR, (secno & 0xFF) as u8);
            port::outb(iobase + ISA_CYL_LO, ((secno >> 8) & 0xFF) as u8);
            port::outb(iobase + ISA_CYL_HI, ((secno >> 16) & 0xFF) as u8);
            port::outb(iobase + ISA_SDH, 0xE0 | ((ideno & 1) << 4) | (((secno >> 24) & 0xF) as u8));
            port::outb(iobase + ISA_COMMAND, IDE_CMD_WRITE);
            //println!("{}",nsecs);
            for i in 0..nsecs {
                //dst = dst + SECTSIZE;
                // if ((ret = ide_wait_ready(iobase, 1)) != 0) {
                // 	goto out;
                // }
                //port::insb(iobase, dst);
                //println!("i={}",i);
                let tmp = &src[(i as usize) * SECTOR_SIZE..((i + 1) as usize) * SECTOR_SIZE];
                if self.ide_wait_ready(iobase, 1) != 0 {
                    println!("wait ready error");
                }
                //println!("write {}:{}",i,src[i as usize]);
                //println!("outsl");
                //port::outsl(iobase, tmp);
                let port = iobase;
                //let buf=&mut buffer;
                for i in 0..tmp.len() {
                    asm!("outsl (%rsi), %dx"
        			:: "{dx}"(port), "{rsi}"(&tmp[i])
        			: "rsi");
                }
                //println!("write :{}",i);
                // for i in 0..4 {
                //  	println!("{}",src[i as usize]);
                // }
                //port::outb(iobase, src[i as usize]);
            }
        }
        Ok(ret)
    }
}

const SECTOR_SIZE: usize = 128;
//const MAX_DMA_SECTORS: usize = 0x2_0000 / SECTOR_SIZE;	// Limited by sector count (and PRDT entries)
const MAX_DMA_SECTORS: usize = 0x1F_F000 / SECTOR_SIZE;    // Limited by sector count (and PRDT entries)
// 512 PDRT entries, assume maximum fragmentation = 512 * 4K max = 2^21 = 2MB per transfer

const HDD_PIO_W28: u8 = 0x30;
const HDD_PIO_R28: u8 = 0x20;
const HDD_PIO_W48: u8 = 0x34;
const HDD_PIO_R48: u8 = 0x24;
const HDD_IDENTIFY: u8 = 0xEC;

const HDD_DMA_R28: u8 = 0xC8;
const HDD_DMA_W28: u8 = 0xCA;
const HDD_DMA_R48: u8 = 0x25;
const HDD_DMA_W48: u8 = 0x35;

const ISA_DATA: u16 = 0x00;
const ISA_ERROR: u16 = 0x01;
const ISA_PRECOMP: u16 = 0x01;
const ISA_CTRL: u16 = 0x02;
const ISA_SECCNT: u16 = 0x02;
const ISA_SECTOR: u16 = 0x03;
const ISA_CYL_LO: u16 = 0x04;
const ISA_CYL_HI: u16 = 0x05;
const ISA_SDH: u16 = 0x06;
const ISA_COMMAND: u16 = 0x07;
const ISA_STATUS: u16 = 0x07;

const IDE_BSY: u8 = 0x80;
const IDE_DRDY: u8 = 0x40;
const IDE_DF: u8 = 0x20;
const IDE_DRQ: u8 = 0x08;
const IDE_ERR: u8 = 0x01;

const IDE_CMD_READ: u8 = 0x20;
const IDE_CMD_WRITE: u8 = 0x30;
const IDE_CMD_IDENTIFY: u8 = 0xEC;

const IDE_IDENT_SECTORS: usize = 20;
const IDE_IDENT_MODEL: usize = 54;
const IDE_IDENT_CAPABILITIES: usize = 98;
const IDE_IDENT_CMDSETS: usize = 164;
const IDE_IDENT_MAX_LBA: usize = 120;
const IDE_IDENT_MAX_LBA_EXT: usize = 200;

const IO_BASE0: u16 = 0x1F0;
const IO_BASE1: u16 = 0x170;
const IO_CTRL0: u16 = 0x3F4;
const IO_CTRL1: u16 = 0x374;

const MAX_IDE: usize = 4;
const MAX_NSECS: usize = 128;
//const MAX_DISK_NSECS          0x10000000U;
//const VALID_IDE(ideno)        (((ideno) >= 0) && ((ideno) < MAX_IDE) && (ide_devices[ideno].valid))

struct Channels {
    base: u16,
    // I/O Base
    ctrl: u16,        // Control Base
}

const CHANNELS: [(u16, u16); 2] = [(IO_BASE0, IO_CTRL0), (IO_BASE1, IO_CTRL1)];

//const IO_BASE(ideno)          (CHANNELS[(ideno) >> 1].base)
//const IO_CTRL(ideno)          (CHANNELS[(ideno) >> 1].ctrl)