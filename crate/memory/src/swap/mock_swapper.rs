use super::Swapper;
use alloc::btree_map::BTreeMap;

pub struct MockSwapper {
    map: BTreeMap<usize, [u8; 4096]>,
}

impl Swapper for MockSwapper {
    fn swap_out(&mut self, data: &[u8; 4096]) -> Result<usize, ()> {
        let id = self.alloc_id();
        self.map.insert(id, data.clone());
        Ok(id)
    }

    fn swap_update(&mut self, token: usize, data: &[u8; 4096]) -> Result<(), ()> {
        if !self.map.contains_key(&token) {
            return Err(());
        }
        self.map.insert(token, data.clone());
        Ok(())
    }
    fn swap_in(&mut self, token: usize, data: &mut [u8; 4096]) -> Result<(), ()> {
        match self.map.remove(&token) {
            Some(d) => *data = d,
            None => return Err(()),
        }
        Ok(())
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

    fn assert_data_eq(data1: &[u8; 4096], data2: &[u8; 4096]) {
        for (&a, &b) in data2.iter().zip(data1.iter()) {
            assert_eq!(a, b);
        }
    }

    #[test]
    fn test() {
        let mut swapper = MockSwapper::new();
        let mut data: [u8; 4096] = unsafe{ uninitialized() };
        let data1: [u8; 4096] = unsafe{ uninitialized() };
        let token = swapper.swap_out(&data1).unwrap();
        swapper.swap_in(token, &mut data);
        assert_data_eq(&data, &data1);

        let data2: [u8; 4096] = unsafe{ uninitialized() };
        swapper.swap_update(token, &data2);
        swapper.swap_in(token, &mut data);
        assert_data_eq(&data, &data2);
    }

    #[test]
    #[should_panic]
    fn invalid_token() {
        let mut swapper = MockSwapper::new();
        let mut data: [u8; 4096] = unsafe{ uninitialized() };
        swapper.swap_in(0, &mut data);
    }
}