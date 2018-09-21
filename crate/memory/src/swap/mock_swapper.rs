use super::Swapper;
use alloc::collections::BTreeMap;
use core::mem::uninitialized;

const PAGE_SIZE: usize = 4096;

#[derive(Default)]
pub struct MockSwapper {
    map: BTreeMap<usize, [u8; PAGE_SIZE]>,
}

impl Swapper for MockSwapper {
    fn swap_out(&mut self, data: &[u8]) -> Result<usize, ()> {
        let id = self.alloc_id();
        let mut slice: [u8; PAGE_SIZE] = unsafe{ uninitialized() };
        slice.copy_from_slice(data);
        self.map.insert(id, slice);
        Ok(id)
    }

    fn swap_update(&mut self, token: usize, data: &[u8]) -> Result<(), ()> {
        if !self.map.contains_key(&token) {
            return Err(());
        }
        let mut slice: [u8; PAGE_SIZE] = unsafe{ uninitialized() };
        slice.copy_from_slice(data);
        self.map.insert(token, slice);
        Ok(())
    }
    fn swap_in(&mut self, token: usize, data: &mut [u8]) -> Result<(), ()> {
        match self.map.remove(&token) {
            Some(d) => data.copy_from_slice(d.as_ref()),
            None => return Err(()),
        }
        Ok(())
    }
}

impl MockSwapper {
    fn alloc_id(&self) -> usize {
        (0 .. 100usize).find(|i| !self.map.contains_key(i)).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn assert_data_eq(data1: &[u8; 4096], data2: &[u8; 4096]) {
        for (&a, &b) in data2.iter().zip(data1.iter()) {
            assert_eq!(a, b);
        }
    }

    #[test]
    fn swap_out_in() {
        let mut swapper = MockSwapper::default();
        let mut data: [u8; 4096] = unsafe{ uninitialized() };
        let data1: [u8; 4096] = unsafe{ uninitialized() };
        let token = swapper.swap_out(&data1).unwrap();
        swapper.swap_in(token, &mut data).unwrap();
        assert_data_eq(&data, &data1);
    }

    #[test]
    fn swap_update() {
        let mut swapper = MockSwapper::default();
        let mut data: [u8; 4096] = unsafe{ uninitialized() };
        let data1: [u8; 4096] = unsafe{ uninitialized() };
        let data2: [u8; 4096] = unsafe{ uninitialized() };
        let token = swapper.swap_out(&data1).unwrap();
        swapper.swap_update(token, &data2).unwrap();
        swapper.swap_in(token, &mut data).unwrap();
        assert_data_eq(&data, &data2);
    }

    #[test]
    fn invalid_token() {
        let mut swapper = MockSwapper::default();
        let mut data: [u8; 4096] = unsafe{ uninitialized() };
        assert_eq!(swapper.swap_in(0, &mut data), Err(()));
    }
}