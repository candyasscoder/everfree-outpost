use core::prelude::*;
use core::ptr;

use physics::v3::{V3, V2, scalar, Region};
use physics::{CHUNK_SIZE, TILE_SIZE};

use types::{StructureTemplate, HAS_ANIM, ModelVertex};

use super::Buffer;


#[derive(Clone, Copy)]
pub struct Vertex {
    // 0
    vert_offset: (u16, u16, u16),
    anim_length: i8,
    anim_rate: u8,

    // 8
    struct_pos: (u8, u8, u8),
    layer: u8,
    display_offset: (u16, u16),

    // 16
    anim_oneshot_start: u16,
    anim_step: u16,
    anim_box_min: (u16, u16),
    anim_box_size: (u16, u16),

    // 28
    _pad1: u32
    // 32
}


pub struct GeomGen<'a> {
    buffer: &'a Buffer<'a>,
    templates: &'a [StructureTemplate],
    model_verts: &'a [ModelVertex],

    bounds: Region<V2>,
    cur: usize,
    sheet: u8,
}

impl<'a> GeomGen<'a> {
    pub unsafe fn init(&mut self,
                       buffer: &'a Buffer<'a>,
                       templates: &'a [StructureTemplate],
                       model_verts: &'a [ModelVertex]) {
        ptr::write(&mut self.buffer, buffer);
        ptr::write(&mut self.templates, templates);
        ptr::write(&mut self.model_verts, model_verts);

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
        while *idx < buf.len() {
            if self.cur >= self.buffer.len {
                // No more structures.
                return false;
            }

            let s = &self.buffer[self.cur];
            self.cur += 1;

            let t = &self.templates[s.template_id as usize];

            let s_pos = V3::new(s.pos.0 as i32,
                                s.pos.1 as i32,
                                s.pos.2 as i32);
            if t.anim_sheet != self.sheet || !t.flags.contains(HAS_ANIM) ||
               !self.bounds.contains(s_pos.reduce()) {
                // Wrong sheet, no anim, or not visible.
                continue;
            }

            if *idx + t.model_length as usize >= buf.len() {
                // Not enough space for this structure's vertices
                break;
            }

            let i0 = t.model_offset as usize;
            let i1 = i0 + t.model_length as usize;
            // Use the offset corresponding to the 0,0,0 corner.
            let display_offset = (t.anim_offset.0 - t.anim_pos.0,
                                  t.anim_offset.1 - t.anim_pos.1 + t.display_size.1 -
                                      t.size.1 as u16 * TILE_SIZE as u16);
            for v in &self.model_verts[i0 .. i1] {
                buf[*idx] = Vertex {
                    vert_offset: (v.x, v.y, v.z),
                    struct_pos: s.pos,
                    layer: t.layer,
                    display_offset: display_offset,
                    anim_length: t.anim_length,
                    anim_rate: t.anim_rate,
                    anim_oneshot_start: s.oneshot_start,
                    anim_step: t.anim_size.0,
                    anim_box_min: t.anim_offset,
                    anim_box_size: t.anim_size,
                    _pad1: 0,
                };
                *idx += 1;
            }
        }

        true
    }
}
