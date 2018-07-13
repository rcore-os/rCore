// Migrate from xv6 mp.c

// Multiprocessor support
// Search memory for MP description structures.
// http://developer.intel.com/design/pentium/datashts/24201606.pdf

mod structs;
use self::structs::*;

/// Search for the MP Floating Pointer Structure, which according to the
/// spec is in one of the following three locations:
/// 1) in the first KB of the EBDA;
/// 2) in the last KB of system base memory;
/// 3) in the BIOS ROM address space between 0F0000h and 0FFFFFh.
pub fn find_mp() -> Option<*const MP>
{
	use core::mem::size_of;
	use util::find_in_memory;
	let ebda = unsafe { *(0x40E as *const u16) as usize } << 4;
	if ebda != 0 {
		let res = unsafe{ find_in_memory::<MP>(ebda, 1024, size_of::<MP>()) };
		if let Some(addr) = res {
			return Some(addr as *const MP);
		}
	}
	let p = unsafe { *(0x413 as *const u16) as usize } << 10;
	let res = unsafe{ find_in_memory::<MP>(p-1024, 1024, size_of::<MP>()) };
    if let Some(addr) = res {
		return Some(addr as *const MP);
	}
	let res = unsafe{ find_in_memory::<MP>(0xF0000, 0x10000, size_of::<MP>()) };
	res.map(|addr| addr as *const MP)
}

/*
struct cpu cpus[NCPU];
int ncpu;
uchar ioapicid;

// Search for an MP configuration table.  For now,
// don't accept the default configurations (physaddr == 0).
// Check for correct signature, calculate the checksum and,
// if correct, check the version.
// To do: check extended table checksum.
static struct mpconf*
mpconfig(struct mp **pmp)
{
	struct mpconf *conf;
	struct mp *mp;

	if((mp = mpsearch()) == 0 || mp->physaddr == 0)
		return 0;
	conf = (struct mpconf*) P2V((uint) mp->physaddr);
	if(memcmp(conf, "PCMP", 4) != 0)
		return 0;
	if(conf->version != 1 && conf->version != 4)
		return 0;
	if(sum((uchar*)conf, conf->length) != 0)
		return 0;
	*pmp = mp;
	return conf;
}

void
mpinit(void)
{
	uchar *p, *e;
	int ismp;
	struct mp *mp;
	struct mpconf *conf;
	struct mpproc *proc;
	struct mpioapic *ioapic;

	if((conf = mpconfig(&mp)) == 0)
		panic("Expect to run on an SMP");
	ismp = 1;
	lapic = (uint*)conf->lapicaddr;
	for(p=(uchar*)(conf+1), e=(uchar*)conf+conf->length; p<e; ){
		switch(*p){
		case MPPROC:
			proc = (struct mpproc*)p;
			if(ncpu < NCPU) {
				cpus[ncpu].apicid = proc->apicid;  // apicid may differ from ncpu
				ncpu++;
			}
			p += sizeof(struct mpproc);
			continue;
		case MPIOAPIC:
			ioapic = (struct mpioapic*)p;
			ioapicid = ioapic->apicno;
			p += sizeof(struct mpioapic);
			continue;
		case MPBUS:
		case MPIOINTR:
		case MPLINTR:
			p += 8;
			continue;
		default:
			ismp = 0;
			break;
		}
	}
	if(!ismp)
		panic("Didn't find a suitable machine");

	if(mp->imcrp){
		// Bochs doesn't support IMCR, so this doesn't run on Bochs.
		// But it would on real hardware.
		outb(0x22, 0x70);   // Select IMCR
		outb(0x23, inb(0x23) | 1);  // Mask external interrupts.
	}
}
*/