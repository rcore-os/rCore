use core::fmt::Debug;

/// Get values by 2 diff keys at the same time
pub trait GetMut2<Idx: Debug + Eq> {
    type Output;
    fn get_mut(&mut self, id: Idx) -> &mut Self::Output;
    fn get_mut2(&mut self, id1: Idx, id2: Idx) -> (&mut Self::Output, &mut Self::Output) {
        assert_ne!(id1, id2);
        let self1 = self as *mut Self;
        let self2 = self1;
        let p1 = unsafe { &mut *self1 }.get_mut(id1);
        let p2 = unsafe { &mut *self2 }.get_mut(id2);
        (p1, p2)
    }
}
