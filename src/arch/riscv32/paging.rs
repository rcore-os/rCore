pub use super::riscv::paging::*;
pub use super::riscv::addr::*;

// need 1 page
pub fn setup_page_table(frame: Frame) {
    let p2 = unsafe { &mut *(frame.start_address().as_u32() as *mut PageTable) };
    p2.zero();

    use self::PageTableFlags as F;
    use consts::{KERNEL_PML4, RECURSIVE_PAGE_PML4};
    // Set recursive map
    p2[RECURSIVE_PAGE_PML4].set(frame.clone(), F::VALID);
    // Set kernel identity map
    p2[KERNEL_PML4].set(Frame::of_addr(PhysAddr::new((KERNEL_PML4 as u32) << 22)), F::VALID | F::READABLE | F::WRITABLE | F::EXCUTABLE);
    p2[KERNEL_PML4 + 1].set(Frame::of_addr(PhysAddr::new((KERNEL_PML4 as u32 + 1) << 22)), F::VALID | F::READABLE | F::WRITABLE | F::EXCUTABLE);

    use super::riscv::register::satp;
    unsafe { satp::set(satp::Mode::Sv32, 0, frame); }
    println!("New page table");
}