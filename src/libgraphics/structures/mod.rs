use core::prelude::*;
use core::ops::{Deref, DerefMut};
use core::ptr;


pub mod base;
pub mod anim;


#[derive(Clone, Copy)]
pub struct Structure {
    /// Structure position in tiles.  u8 is enough to cover the entire local region.
    pub pos: (u8, u8, u8),

    pub external_id: u32,

    pub template_id: u16,

    /// Timestamp indicating when to start the structure's one-shot animation.  This field is only
    /// relevant if the structure's template defines such an animation.
    pub oneshot_start: u16,
}


pub struct Buffer<'a> {
    /// Some space to store the actual structure data.
    storage: &'a mut [Structure],

    /// The number of structures currently present in the buffer.  Slots `0 .. len` of `storage`
    /// contain valid data.
    len: usize,
}

impl<'a> Buffer<'a> {
    pub unsafe fn init(&mut self, storage: &'a mut [Structure]) {
        ptr::write(&mut self.storage, storage);
        self.len = 0;
    }

    pub fn insert(&mut self,
                  external_id: u32,
                  pos: (u8, u8, u8),
                  template_id: u32) -> Option<usize> {
        if self.len >= self.storage.len() {
            // No space for more structures.
            return None;
        }

        let idx = self.len;
        self.storage[idx] = Structure {
            pos: pos,
            external_id: external_id,
            template_id: template_id as u16,
            oneshot_start: 0,
        };
        self.len += 1;

        Some(idx)
    }

    pub fn remove(&mut self,
                  idx: usize) -> u32 {
        // Do a sort of `swap_remove`, except we don't need to return the old value.
        self.storage[idx] = self.storage[self.len - 1];
        self.len -= 1;

        // Return the external ID of the structure that now occupies slot `idx`, so the caller can
        // update their records.
        self.storage[idx].external_id
    }
}

impl<'a> Deref for Buffer<'a> {
    type Target = [Structure];

    fn deref(&self) -> &[Structure] {
        &self.storage[.. self.len]
    }
}

impl<'a> DerefMut for Buffer<'a> {
    fn deref_mut(&mut self) -> &mut [Structure] {
        &mut self.storage[.. self.len]
    }
}
