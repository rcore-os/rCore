use core::fmt::Debug;

/// Convert C string to Rust string
pub unsafe fn from_cstr(s: *const u8) -> &'static str {
    use core::{str, slice};
    let len = (0usize..).find(|&i| *s.offset(i as isize) == 0).unwrap();
    str::from_utf8(slice::from_raw_parts(s, len)).unwrap()
}
