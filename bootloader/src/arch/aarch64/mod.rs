use core::slice;
use fixedvec::FixedVec;
use xmas_elf::program::{ProgramHeader64, Type};

global_asm!(include_str!("boot.S"));

pub fn map_kernel(kernel_start: u64, segments: &FixedVec<ProgramHeader64>) {
    for segment in segments {
        if segment.get_type() != Ok(Type::Load) {
            continue;
        }
        let virt_addr = segment.virtual_addr;
        let offset = segment.offset;
        let file_size = segment.file_size as usize;
        let mem_size = segment.mem_size as usize;

        unsafe {
            let target = slice::from_raw_parts_mut(virt_addr as *mut u8, mem_size);
            let source = slice::from_raw_parts((kernel_start + offset) as *const u8, file_size);
            target.copy_from_slice(source);
            target[file_size..].iter_mut().for_each(|x| *x = 0);
        }
    }
}
