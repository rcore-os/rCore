use super::*;
use crate::lkm::manager::ModuleManager;
use crate::lkm::structs::LoadedModule;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use core::alloc::{GlobalAlloc, Layout};
use core::slice::from_raw_parts;

pub fn get_module(this_module: usize) -> &'static mut LoadedModule {
    unsafe {
        let ptr = this_module as *mut LoadedModule;
        &mut (*ptr) as (&'static mut LoadedModule)
    }
}

pub unsafe fn cstr_to_str(ptr: *const u8, max_size: usize) -> String {
    (0..max_size)
        .find(|&i| ptr.offset(i as isize).read() == 0)
        .and_then(|len| core::str::from_utf8(core::slice::from_raw_parts(ptr, len)).ok())
        .map(|s| String::from(s))
        .unwrap()
}

#[no_mangle]
pub extern "C" fn lkm_api_pong() -> usize {
    println!("Pong from Kernel Module!");
    println!(
        "This indicates that a kernel module is successfully loaded into kernel and called a stub."
    );
    114514
}

#[no_mangle]
pub extern "C" fn lkm_api_debug(this_module: usize) {
    let module = get_module(this_module);
    module.lock.lock();
    println!(
        "[LKM] Current module info: name={} version={} api_version={}\nref_count={} dep_count={}",
        module.info.name,
        module.info.version,
        module.info.api_version,
        Arc::strong_count(&module.using_counts),
        module.used_counts
    );
}

#[no_mangle]
pub extern "C" fn lkm_api_query_symbol(symbol: *const u8) -> usize {
    manager::ModuleManager::with(|man| {
        match man.resolve_symbol(&unsafe { cstr_to_str(symbol, 256) }) {
            Some(x) => x,
            None => 0,
        }
    })
}

#[no_mangle]
pub extern "C" fn lkm_api_kmalloc(size: usize) -> usize {
    unsafe { crate::HEAP_ALLOCATOR.alloc(Layout::from_size_align(size, 8).unwrap()) as usize }
}

#[no_mangle]
pub extern "C" fn lkm_api_kfree(ptr: usize, size: usize) {
    unsafe {
        crate::HEAP_ALLOCATOR.dealloc(ptr as *mut u8, Layout::from_size_align(size, 8).unwrap());
    }
}

#[no_mangle]
pub extern "C" fn lkm_api_info(ptr: *const u8) {
    let text = unsafe { cstr_to_str(ptr, 1024) };
    info!("{}", text);
}

#[no_mangle]
pub extern "C" fn lkm_api_add_kernel_symbols(start: usize, end: usize) {
    use crate::lkm::manager::LKM_MANAGER;
    let length = end - start;
    use core::str::from_utf8;
    let symbols = unsafe { from_utf8(from_raw_parts(start as *const u8, length)) }.unwrap();
    let global_lkmm = &LKM_MANAGER;
    let mut locked_lkmm = global_lkmm.lock();
    let mut lkmm = locked_lkmm.as_mut().unwrap();
    lkmm.init_kernel_symbols(symbols);
}
