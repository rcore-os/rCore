use trapframe::UserContext;

// mcontext_t
#[repr(C)]
#[derive(Clone, Debug)]
pub struct MachineContext {
    // gregs
    pub zero: usize,
    pub ra: usize,
    pub sp: usize,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub s0: usize,
    pub s1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
}

impl MachineContext {
    pub fn from_tf(tf: &UserContext) -> Self {
        Self {
            zero: tf.general.zero,
            ra: tf.general.ra,
            sp: tf.general.sp,
            gp: tf.general.gp,
            tp: tf.general.tp,
            t0: tf.general.t0,
            t1: tf.general.t1,
            t2: tf.general.t2,
            s0: tf.general.s0,
            s1: tf.general.s1,
            a0: tf.general.a1,
            a1: tf.general.a1,
            a2: tf.general.a2,
            a3: tf.general.a3,
            a4: tf.general.a4,
            a5: tf.general.a5,
            a6: tf.general.a6,
            a7: tf.general.a7,
            s2: tf.general.s2,
            s3: tf.general.s3,
            s4: tf.general.s4,
            s5: tf.general.s5,
            s6: tf.general.s6,
            s7: tf.general.s7,
            s8: tf.general.s8,
            s9: tf.general.s9,
            s10: tf.general.s10,
            s11: tf.general.s11,
            t3: tf.general.t3,
            t4: tf.general.t4,
            t5: tf.general.t5,
            t6: tf.general.t6,
        }
    }

    pub fn fill_tf(&self, ctx: &mut UserContext) {}
}
