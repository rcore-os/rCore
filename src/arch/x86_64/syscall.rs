use arch::interrupt::consts::*;

pub fn switch_to_user() {
    unsafe { int!(T_SWITCH_TOU); }
}

pub fn switch_to_kernel() {
    unsafe { int!(T_SWITCH_TOK); }
}