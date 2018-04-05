pub fn bytes_sum<T>(p: &T) -> u8 {
	use core::mem::size_of_val;
	let len = size_of_val(p);
	let p = p as *const T as *const u8;
	(0..len).map(|i| unsafe { &*p.offset(i as isize) })
		.fold(0, |a, &b| a.overflowing_add(b).0)
}

/// 
pub trait Checkable {
	fn check(&self) -> bool;
}

/// Scan memory to find the struct
pub unsafe fn find_in_memory<T: Checkable>
	(begin: usize, len: usize, step: usize) -> Option<usize> {

	(begin .. begin + len).step_by(step)
		.find(|&addr| { (&*(addr as *const T)).check() })
}
