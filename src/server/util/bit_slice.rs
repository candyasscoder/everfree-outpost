use std::mem;


pub struct BitSlice([u8]);

impl BitSlice {
    pub fn from_bytes<'a>(bytes: &'a [u8]) -> &'a BitSlice {
        unsafe { mem::transmute(bytes) }
    }

    pub fn from_bytes_mut<'a>(bytes: &'a mut [u8]) -> &'a mut BitSlice {
        unsafe { mem::transmute(bytes) }
    }

    pub fn len(&self) -> usize {
        8 * self.0.len()
    }

    pub fn get(&self, idx: usize) -> bool {
        let byte = idx >> 3;
        let offset = idx & 7;
        self.0[byte] & (1 << offset) != 0
    }

    pub fn set(&mut self, idx: usize, val: bool) {
        let byte = idx >> 3;
        let offset = idx & 7;
        let bit = 1 << offset;
        if val {
            self.0[byte] |= bit;
        } else {
            self.0[byte] &= !bit;
        }
    }
}
