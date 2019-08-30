use bootinfo::BootInfo;
use core::ptr;
use fixedvec::FixedVec;
use xmas_elf::program::{ProgramHeader32, Type};

const KERNEL_OFFSET: u32 = 0x80000000;

global_asm!(include_str!("boot.S"));

pub fn copy_kernel(kernel_start: usize, segments: &FixedVec<ProgramHeader32>) -> (BootInfo, usize) {
    // reverse program headers to avoid overlapping in memory copying
    let mut space = alloc_stack!([ProgramHeader32; 32]);
    let mut rev_segments = FixedVec::new(&mut space);
    for i in (0..segments.len()).rev() {
        rev_segments.push(segments[i]).unwrap();
    }

    let mut end_vaddr = 0;
    for segment in &rev_segments {
        if segment.get_type() != Ok(Type::Load) {
            continue;
        }
        let virt_addr = segment.virtual_addr;
        let offset = segment.offset;
        let file_size = segment.file_size;
        let mem_size = segment.mem_size;

        unsafe {
            let src = (kernel_start as u32 + offset) as *const u8;
            let dst = virt_addr.wrapping_sub(KERNEL_OFFSET) as *mut u8;
            ptr::copy(src, dst, file_size as usize);
            ptr::write_bytes(dst.offset(file_size as isize), 0, (mem_size - file_size) as usize);
        }
        if virt_addr + mem_size > end_vaddr {
            end_vaddr = virt_addr + mem_size;
        }
    }

    (
        BootInfo {
            dtb: include_bytes!(concat!("../../../", env!("DTB"))).as_ptr() as usize,
        },
        end_vaddr as usize,
    )
}
