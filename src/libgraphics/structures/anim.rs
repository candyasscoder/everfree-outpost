use core::prelude::*;
use core::ptr;

use physics::v3::{V3, V2, scalar, Region};
use physics::CHUNK_SIZE;

use IntrusiveCorner;
use {emit_quad, remaining_quads};
use types::{StructureTemplate, HAS_ANIM};

use super::Buffer;


#[derive(Clone, Copy)]
pub struct Vertex {
    // 0
    corner: (u8, u8),
    pos: (u8, u8, u8),
    layer: u8,
    _pad1: u16,

    // 8
    anim_size: (u16, u16),
    anim_offset: (u16, u16),
    anim_pos: (u16, u16),
    anim_length: i8,
    anim_rate: u8,
    anim_oneshot_start: u16,

    // 24
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

            let s_pos = V3::new(s.pos.0 as i32,
                                s.pos.1 as i32,
                                s.pos.2 as i32);
            if t.sheet != self.sheet || !t.flags.contains(HAS_ANIM) ||
               !self.bounds.contains(s_pos.reduce()) {
                continue;
            }


            emit_quad(buf, idx, Vertex {
                corner: (0, 0),
                // Give the position of the top front corner of the structure, since the quad should
                // cover the front plane.
                pos: (s.pos.0,
                      s.pos.1 + t.size.1,
                      s.pos.2),
                layer: t.layer,
                _pad1: 0,

                anim_offset: t.anim_offset,
                // Give the anim_pos relative to the bottom corner.  This makes life easier for the
                // shader.
                anim_pos: (t.anim_pos.0,
                           t.display_size.1 - (t.anim_pos.1 + t.anim_size.1 as u16)),
                anim_size: t.anim_size,
                anim_length: t.anim_length,
                anim_rate: t.anim_rate,
                anim_oneshot_start: s.oneshot_start,
            });
        }

        true
    }
}
