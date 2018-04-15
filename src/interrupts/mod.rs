use x86_64::VirtualAddress;
use x86_64::structures::idt::Idt;
use x86_64::structures::tss::TaskStateSegment;
use memory::MemoryController;
use spin::Once;

mod gdt;
mod irq;

// Copied from xv6 x86_64
const GNULL: gdt::Descriptor = gdt::Descriptor::UserSegment(0);
const KCODE: gdt::Descriptor = gdt::Descriptor::UserSegment(0x0020980000000000);  // EXECUTABLE | USER_SEGMENT | PRESENT | LONG_MODE
const UCODE: gdt::Descriptor = gdt::Descriptor::UserSegment(0x0020F80000000000);  // EXECUTABLE | USER_SEGMENT | USER_MODE | PRESENT | LONG_MODE
const KDATA: gdt::Descriptor = gdt::Descriptor::UserSegment(0x0000920000000000);  // DATA_WRITABLE | USER_SEGMENT | PRESENT
const UDATA: gdt::Descriptor = gdt::Descriptor::UserSegment(0x0000F20000000000);  // DATA_WRITABLE | USER_SEGMENT | USER_MODE | PRESENT

static TSS: Once<TaskStateSegment> = Once::new();
static GDT: Once<gdt::Gdt> = Once::new();
static IDT: Once<Idt> = Once::new();

const DOUBLE_FAULT_IST_INDEX: usize = 0;

pub fn init(memory_controller: &mut MemoryController) {

    test::print_flags();

    use x86_64::structures::gdt::SegmentSelector;
    use x86_64::instructions::segmentation::set_cs;
    use x86_64::instructions::tables::load_tss;

    let double_fault_stack = memory_controller.alloc_stack(1)
        .expect("could not allocate double fault stack");

    let tss = TSS.call_once(|| {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX] = VirtualAddress(
            double_fault_stack.top());
        tss
    });

    let mut code_selector = SegmentSelector(0);
    let mut tss_selector = SegmentSelector(0);
    let gdt = GDT.call_once(|| {
        let mut gdt = gdt::Gdt::new();
        gdt.add_entry(GNULL);
        code_selector = 
        gdt.add_entry(KCODE);
        gdt.add_entry(UCODE);
        gdt.add_entry(KDATA);
        gdt.add_entry(UDATA);
        tss_selector = gdt.add_entry(gdt::Descriptor::tss_segment(&tss));
        gdt
    });
    gdt.load();

    unsafe {
        // reload code segment register
        set_cs(code_selector);
        // load TSS
        load_tss(tss_selector);
    }

    let idt = IDT.call_once(|| {
        use self::irq::*;
        use consts::irq::*;
        
        let mut idt = Idt::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);
        idt[(T_IRQ0 + IRQ_COM1) as usize].set_handler_fn(serial_handler);
        idt[(T_IRQ0 + IRQ_KBD) as usize].set_handler_fn(keyboard_handler);
        idt[(T_IRQ0 + IRQ_TIMER) as usize].set_handler_fn(timer_handler);
        unsafe {
            idt.page_fault.set_handler_fn(page_fault_handler)
                .set_stack_index(DOUBLE_FAULT_IST_INDEX as u16);
        }
        idt
    });

    idt.load();
}

pub mod test
{
    pub fn print_flags() {
        use super::gdt::*;
        // The following 4 GDT entries were copied from xv6 x86_64
        let list: [(&str, Descriptor); 4] = [
            ("KCODE", super::KCODE), // Code, DPL=0, R/X
            ("UCODE", super::UCODE), // Code, DPL=3, R/X
            ("KDATA", super::KDATA), // Data, DPL=0, W
            ("UDATA", super::UDATA), // Data, DPL=3, W
        ];
        // Let's see what that means
        println!("GDT Segments from xv6 x86_64:");
        for (name, desc) in list.iter() {
            println!("  {}: {:?}", name, desc);
        }
    }
}