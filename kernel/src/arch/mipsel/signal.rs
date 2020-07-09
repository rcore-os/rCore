use crate::signal::Siginfo;
use crate::signal::SignalUserContext;
use trapframe::UserContext;

// mcontext
#[repr(C)]
#[derive(Clone, Debug)]
pub struct MachineContext {}

impl MachineContext {
    pub fn from_tf(tf: &UserContext) -> Self {
        Self {}
    }

    pub fn fill_tf(&self, tf: &mut UserContext) {}
}

// TODO
pub const RET_CODE: [u8; 7] = [0; 7];

pub fn set_signal_handler(
    tf: &mut UserContext,
    sp: usize,
    handler: usize,
    signo: usize,
    siginfo: *const Siginfo,
    ucontext: *const SignalUserContext,
) {
    //tf.sp = sp;
    //tf.elr = handler;

    // pass handler argument
    //tf.general.x0 = signo as usize;
    //tf.general.x1 = siginfo as usize;
    //tf.general.x2 = ucontext as usize;
}
