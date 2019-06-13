use alloc::collections::btree_map::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::ptr::null;

pub struct ProcInitInfo {
    pub args: Vec<String>,
    pub envs: Vec<String>,
    pub auxv: BTreeMap<u8, usize>,
}

impl ProcInitInfo {
    pub unsafe fn push_at(&self, stack_top: usize) -> usize {
        let mut writer = StackWriter { sp: stack_top };
        // from stack_top:
        // program name
        writer.push_str(&self.args[0]);
        // environment strings
        let envs: Vec<_> = self
            .envs
            .iter()
            .map(|arg| {
                writer.push_str(arg.as_str());
                writer.sp
            })
            .collect();
        // argv strings
        let argv: Vec<_> = self
            .args
            .iter()
            .map(|arg| {
                writer.push_str(arg.as_str());
                writer.sp
            })
            .collect();
        // auxiliary vector entries
        writer.push_slice(&[null::<u8>(), null::<u8>()]);
        for (&type_, &value) in self.auxv.iter() {
            writer.push_slice(&[type_ as usize, value]);
        }
        // envionment pointers
        writer.push_slice(&[null::<u8>()]);
        writer.push_slice(envs.as_slice());
        // argv pointers
        writer.push_slice(&[null::<u8>()]);
        writer.push_slice(argv.as_slice());
        // argc
        writer.push_slice(&[argv.len()]);
        writer.sp
    }
}

struct StackWriter {
    sp: usize,
}

impl StackWriter {
    fn push_slice<T: Copy>(&mut self, vs: &[T]) {
        use core::{
            mem::{align_of, size_of},
            slice,
        };
        self.sp -= vs.len() * size_of::<T>();
        self.sp -= self.sp % align_of::<T>();
        unsafe { slice::from_raw_parts_mut(self.sp as *mut T, vs.len()) }.copy_from_slice(vs);
    }
    fn push_str(&mut self, s: &str) {
        self.push_slice(&[b'\0']);
        self.push_slice(s.as_bytes());
    }
}

pub const AT_PHDR: u8 = 3;
pub const AT_PHENT: u8 = 4;
pub const AT_PHNUM: u8 = 5;
pub const AT_PAGESZ: u8 = 6;
pub const AT_BASE: u8 = 7;
pub const AT_ENTRY: u8 = 9;
