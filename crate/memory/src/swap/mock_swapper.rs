use super::Swapper;
use alloc::btree_map::BTreeMap;

pub struct MockSwapper {
    map: BTreeMap<usize, [u8; 4096]>,
}

impl Swapper for MockSwapper {
    fn swap_out(&mut self, data: &[u8; 4096]) -> usize {
        let id = self.alloc_id();
        self.map.insert(id, data.clone());
        id
    }

    fn swap_in(&mut self, token: usize, data: &mut [u8; 4096]) {
        *data = self.map.remove(&token).unwrap();
    }
}

impl MockSwapper {
    pub fn new() -> Self {
        MockSwapper {map: BTreeMap::new()}
    }
    fn alloc_id(&self) -> usize {
        (0 .. 100usize).find(|i| !self.map.contains_key(i)).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use core::mem::uninitialized;

    #[test]
    fn test() {
        let mut swapper = MockSwapper::new();
        let data: [u8; 4096] = unsafe{ uninitialized() };
        let mut data1: [u8; 4096] = unsafe{ uninitialized() };
        let token = swapper.swap_out(&data);
        swapper.swap_in(token, &mut data1);
        for (&a, &b) in data.iter().zip(data1.iter()) {
            assert_eq!(a, b);
        }
    }

    #[test]
    #[should_panic]
    fn invalid_token() {
        let mut swapper = MockSwapper::new();
        let mut data: [u8; 4096] = unsafe{ uninitialized() };
        swapper.swap_in(0, &mut data);
    }
}