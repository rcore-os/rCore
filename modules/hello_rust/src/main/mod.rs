extern crate rcore;
extern crate alloc;
use rcore::lkm::api::lkm_api_pong;
use alloc::vec::Vec;

pub mod hello;
#[no_mangle]
pub extern "C" fn init_module(){
    lkm_api_pong();
    let mut v: Vec<u8>=Vec::new();
    v.push(10);
    v.push(20);
    hello::hello_again();
}

