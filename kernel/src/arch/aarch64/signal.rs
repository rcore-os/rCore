use trapframe::UserContext;

// mcontext
#[repr(C)]
#[derive(Clone, Debug)]
pub struct MachineContext {
    fault_address: usize,
    x0: usize,
    x1: usize,
    x2: usize,
    x3: usize,
    x4: usize,
    x5: usize,
    x6: usize,
    x7: usize,
    x8: usize,
    x9: usize,
    x10: usize,
    x11: usize,
    x12: usize,
    x13: usize,
    x14: usize,
    x15: usize,
    x16: usize,
    x17: usize,
    x18: usize,
    x19: usize,
    x20: usize,
    x21: usize,
    x22: usize,
    x23: usize,
    x24: usize,
    x25: usize,
    x26: usize,
    x27: usize,
    x28: usize,
    x29: usize,
    x30: usize,
    sp: usize,
    pc: usize,
    pstate: usize,
}

impl MachineContext {
    pub fn from_tf(tf: &UserContext) -> Self {
        Self {
            fault_address: 0,
            x0: tf.general.x0,
            x1: tf.general.x1,
            x2: tf.general.x2,
            x3: tf.general.x3,
            x4: tf.general.x4,
            x5: tf.general.x5,
            x6: tf.general.x6,
            x7: tf.general.x7,
            x8: tf.general.x8,
            x9: tf.general.x9,
            x10: tf.general.x10,
            x11: tf.general.x13,
            x12: tf.general.x12,
            x13: tf.general.x13,
            x14: tf.general.x14,
            x15: tf.general.x15,
            x16: tf.general.x16,
            x17: tf.general.x17,
            x18: tf.general.x18,
            x19: tf.general.x19,
            x20: tf.general.x20,
            x21: tf.general.x21,
            x22: tf.general.x22,
            x23: tf.general.x23,
            x24: tf.general.x24,
            x25: tf.general.x25,
            x26: tf.general.x26,
            x27: tf.general.x27,
            x28: tf.general.x28,
            x29: tf.general.x29,
            x30: tf.general.x30,
            sp: tf.sp,
            pc: tf.elr,
            pstate: tf.spsr,
        }
    }

    pub fn fill_tf(&self, tf: &mut UserContext) {
        tf.general.x0 = self.x0;
        tf.general.x1 = self.x1;
        tf.general.x2 = self.x2;
        tf.general.x3 = self.x3;
        tf.general.x4 = self.x4;
        tf.general.x5 = self.x5;
        tf.general.x6 = self.x6;
        tf.general.x7 = self.x7;
        tf.general.x8 = self.x8;
        tf.general.x9 = self.x9;
        tf.general.x10 = self.x10;
        tf.general.x11 = self.x11;
        tf.general.x12 = self.x12;
        tf.general.x13 = self.x13;
        tf.general.x14 = self.x14;
        tf.general.x15 = self.x15;
        tf.general.x16 = self.x16;
        tf.general.x17 = self.x17;
        tf.general.x18 = self.x18;
        tf.general.x19 = self.x19;
        tf.general.x20 = self.x20;
        tf.general.x21 = self.x21;
        tf.general.x22 = self.x22;
        tf.general.x23 = self.x23;
        tf.general.x24 = self.x24;
        tf.general.x25 = self.x25;
        tf.general.x26 = self.x26;
        tf.general.x27 = self.x27;
        tf.general.x28 = self.x28;
        tf.general.x29 = self.x29;
        tf.general.x30 = self.x30;
        tf.sp = self.sp;
        tf.elr = self.pc;
        tf.spsr = self.pstate;
    }
}
