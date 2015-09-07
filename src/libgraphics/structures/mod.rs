use core::prelude::*;
use core::ops::{Deref, DerefMut};
use core::ptr;

use physics::v3::{V3, V2, Region};
use physics::CHUNK_BITS;

use LOCAL_BITS;
use types::StructureTemplate;


pub mod base;
//mod anim;


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


fn overlaps_wrapping(a: Region<V2>, b: Region<V2>) -> bool {
    //        |--A--|       `a` region
    // |-B-|                `b` region
    // |--------|           local region
    // A--|   |--           `a` region, wrapped
    //
    // The normal check is `b.min < a.max && a.min < b.max`.  This check fails for the `a` and `b`
    // in the example.  We instead do a "wrapped check": `(x - a.min) & MASK < a.max - a.min`.
    // This only works correctly when testing a single point (`x`) against a range, so we test each
    // endpoint against the other range and return true if any lies within.

    overlaps_wrapping_1d((a.min.x, a.max.x), (b.min.x, b.max.x)) &&
    overlaps_wrapping_1d((a.min.y, a.max.y), (b.min.y, b.max.y))
}

fn overlaps_wrapping_1d(a: (i32, i32), b: (i32, i32)) -> bool {
    const MASK: i32 = (1 << (LOCAL_BITS + CHUNK_BITS)) - 1;
    let (a_min, a_max) = a;
    let (b_min, b_max) = b;

    (b_min - a_min) & MASK < a_max - a_min ||
    (b_max - a_min) & MASK < a_max - a_min ||
    (a_min - b_min) & MASK < b_max - b_min ||
    (a_max - b_min) & MASK < b_max - b_min
}


fn check_output(s: &Structure,
                t: &StructureTemplate,
                bounds: Region<V2>,
                sheet: u8) -> bool {
    if t.sheet != sheet {
        return false;
    }


    let pos = V3::new(s.pos.0 as i32,
                      s.pos.1 as i32,
                      s.pos.2 as i32);
    let size = V3::new(t.size.0 as i32,
                       t.size.1 as i32,
                       t.size.2 as i32);
    let draw_min = V2::new(pos.x,
                           pos.y - pos.z - size.z);
    let draw_max = V2::new(pos.x + size.x,
                           pos.y - pos.z + size.y);
    let draw_bounds = Region::new(draw_min, draw_max);

    overlaps_wrapping(draw_bounds, bounds)
}
