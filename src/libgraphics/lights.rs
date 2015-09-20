use core::prelude::*;
use core::ptr;

use physics::v3::{V3, V2, scalar, Region};
use physics::{CHUNK_BITS, CHUNK_SIZE, TILE_BITS, TILE_SIZE};

use IntrusiveCorner;
use {emit_quad, remaining_quads};
use LOCAL_BITS;
use types::{StructureTemplate, HAS_LIGHT};
use structures::Buffer;


#[derive(Clone, Copy)]
pub struct Vertex {
    // 0
    corner: (u8, u8),
    center: (u16, u16, u16),

    // 8
    color: (u8, u8, u8),
    _pad1: u8,
    radius: u16,
    _pad2: u16,

    // 16
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
}

impl<'a> GeomGen<'a> {
    pub unsafe fn init(&mut self,
                       buffer: &'a Buffer<'a>,
                       templates: &'a [StructureTemplate]) {
        ptr::write(&mut self.buffer, buffer);
        ptr::write(&mut self.templates, templates);

        ptr::write(&mut self.bounds, Region::new(scalar(0), scalar(0)));
        self.cur = 0;
    }

    pub fn reset(&mut self, chunk_bounds: Region<V2>) {
        self.bounds = chunk_bounds * scalar(CHUNK_SIZE * TILE_SIZE);
        self.cur = 0;
    }

    pub fn generate(&mut self,
                    buf: &mut [Vertex],
                    idx: &mut usize) -> bool {
        while remaining_quads(buf, *idx) >= 1 {
            if self.cur >= self.buffer.len() {
                return false;
            }

            let s = &self.buffer[self.cur];
            self.cur += 1;

            let t = &self.templates[s.template_id as usize];

            if !t.flags.contains(HAS_LIGHT) {
                continue;
            }

            // Be careful to avoid emitting duplicate geometry.  Two copies of a structure looks
            // the same as one, but two copies of a light is twice as bright.
            let offset = V3::new(t.light_pos.0 as i32,
                                 t.light_pos.1 as i32,
                                 t.light_pos.2 as i32);
            let s_pos = V3::new(s.pos.0 as i32,
                                s.pos.1 as i32,
                                s.pos.2 as i32);
            let center = s_pos * scalar(TILE_SIZE) + offset;

            // Do a wrapped version of `self.bounds.contains(center)`.
            const MASK: i32 = (1 << (LOCAL_BITS + CHUNK_BITS + TILE_BITS)) - 1;
            let wrapped_center = (center.reduce() - self.bounds.min) & scalar(MASK);
            let wrapped_bounds = self.bounds - self.bounds.min;
            if !wrapped_bounds.contains(wrapped_center) {
                continue;
            }

            emit_quad(buf, idx, Vertex {
                corner: (0, 0),
                // Give the position of the front corner of the structure, since the quad should
                // cover the front plane.
                center: (center.x as u16,
                         center.y as u16,
                         center.z as u16),

                color: t.light_color,
                radius: t.light_radius,

                _pad1: 0,
                _pad2: 0,
            });
        }

        true
    }
}
