use core::prelude::*;
use core::ops::{Deref, DerefMut};
use core::ptr;

use physics::v3::{V3, V2, scalar, Region};
use physics::CHUNK_SIZE;

use IntrusiveCorner;
use {emit_quad, remaining_quads};
use types::StructureTemplate;


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


#[derive(Clone, Copy)]
pub struct Vertex {
    // 0
    corner: (u8, u8),
    pos: (u8, u8, u8),
    layer: u8,
    _pad1: u16,

    // 8
    display_size: (u16, u16),
    display_offset: (u16, u16),

    // 12
}

impl IntrusiveCorner for Vertex {
    fn corner(&self) -> &(u8, u8) { &self.corner }
    fn corner_mut(&mut self) -> &mut (u8, u8) { &mut self.corner }
}

pub struct GeomGen<'a> {
    buffer: &'a Buffer<'a>,
    templates: &'a [StructureTemplate],

    bounds: Region<V2>,
    cur: usize,
    sheet: u8,
}

impl<'a> GeomGen<'a> {
    pub unsafe fn init(&mut self,
                       buffer: &'a Buffer<'a>,
                       templates: &'a [StructureTemplate]) {
        ptr::write(&mut self.buffer, buffer);
        ptr::write(&mut self.templates, templates);

        ptr::write(&mut self.bounds, Region::new(scalar(0), scalar(0)));
        self.cur = 0;
        self.sheet = 0;
    }

    pub fn reset(&mut self, chunk_bounds: Region<V2>, sheet: u8) {
        self.bounds = chunk_bounds * scalar(CHUNK_SIZE);
        self.cur = 0;
        self.sheet = sheet;
    }

    pub fn generate(&mut self,
                    buf: &mut [Vertex],
                    idx: &mut usize) -> bool {
        while remaining_quads(buf, *idx) >= 1 {
            if self.cur >= self.buffer.len {
                return false;
            }

            let s = &self.buffer[self.cur];
            self.cur += 1;

            let t = &self.templates[s.template_id as usize];

            if t.sheet != self.sheet {
                continue;
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


            if !overlaps_wrapping(draw_bounds, self.bounds) {
                continue;
            }


            emit_quad(buf, idx, Vertex {
                corner: (0, 0),
                pos: s.pos,
                layer: t.layer,
                _pad1: 0,
                display_size: t.display_size,
                display_offset: t.display_offset,
            });
        }

        true
    }
}

fn overlaps_wrapping(_a: Region<V2>, _b: Region<V2>) -> bool {
    // TODO
    true
}
