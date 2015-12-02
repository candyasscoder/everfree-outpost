use core::prelude::*;
use core::ptr;

use physics::v3::{V3, V2, scalar, Region};
use physics::{CHUNK_SIZE, TILE_SIZE};

use types::{StructureTemplate, HAS_ANIM, TemplatePart, TemplateVertex};

use super::Buffer;


#[derive(Clone, Copy)]
pub struct Vertex {
    // 0
    vert_offset: (u16, u16, u16),
    _pad1: u16,

    // 8
    struct_pos: (u8, u8, u8),
    layer: u8,
    display_offset: (u16, u16),

    // 16
}

pub struct GeomGen<'a> {
    buffer: &'a Buffer<'a>,
    templates: &'a [StructureTemplate],
    parts: &'a [TemplatePart],
    verts: &'a [TemplateVertex],

    bounds: Region<V2>,
    cur: usize,
    sheet: u8,
}

impl<'a> GeomGen<'a> {
    pub unsafe fn init(&mut self,
                       buffer: &'a Buffer<'a>,
                       templates: &'a [StructureTemplate],
                       parts: &'a [TemplatePart],
                       verts: &'a [TemplateVertex]) {
        ptr::write(&mut self.buffer, buffer);
        ptr::write(&mut self.templates, templates);
        ptr::write(&mut self.parts, parts);
        ptr::write(&mut self.verts, verts);

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
                // No more structures
                return false;
            }

            let s = &self.buffer[self.cur];
            self.cur += 1;

            let t = &self.templates[s.template_id as usize];

            let s_pos = V3::new(s.pos.0 as i32,
                                s.pos.1 as i32,
                                s.pos.2 as i32);
            if t.sheet != self.sheet || !self.bounds.contains(s_pos.reduce()) {
                // Wrong sheet, or not visible
                continue;
            }

            if *idx + t.vert_count as usize >= buf.len() {
                // Not enough space for all this structure's vertices.
                // Note the structure may not render all its vertices right now (some may be for
                // animated parts), but it's always better to be safe.
                break;
            }

            let i0 = t.part_idx as usize;
            let i1 = i0 + t.part_count as usize;
            for p in &self.parts[i0 .. i1] {
                if p.flags.contains(HAS_ANIM) {
                    continue;
                }

                let j0 = p.vert_idx as usize;
                let j1 = j0 + p.vert_count as usize;
                for v in &self.verts[j0 .. j1] {
                    buf[*idx] = Vertex {
                        vert_offset: (v.x, v.y, v.z),
                        _pad1: 0,
                        struct_pos: s.pos,
                        layer: t.layer,
                        display_offset: p.offset,
                    };
                    *idx += 1;
                }
            }
        }

        true
    }
}
