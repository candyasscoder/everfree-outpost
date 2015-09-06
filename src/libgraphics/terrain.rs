use core::prelude::*;
use core::ptr;

use physics::v3::{V3, V2, Vn, scalar, Region, RegionPoints};
use physics::CHUNK_SIZE;
use ATLAS_SIZE;

use IntrusiveCorner;
use {emit_quad, remaining_quads};
use block_data::BlockData;
use LocalChunks;


const LOCAL_SIZE: i32 = 8;


/// Vertex attributes for terrain.
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct Vertex {
    corner: (u8, u8),
    pos: (u8, u8, u8),
    side: u8,
    tex_coord: (u8, u8),
}

impl IntrusiveCorner for Vertex {
    fn corner(&self) -> &(u8, u8) { &self.corner }
    fn corner_mut(&mut self) -> &mut (u8, u8) { &mut self.corner }
}


pub struct GeomGen<'a> {
    block_data: &'a [BlockData],
    local_chunks: &'a LocalChunks,

    cpos: V2,
    iter: RegionPoints<V3>,
}

impl<'a> GeomGen<'a> {
    pub unsafe fn init(&mut self,
                       block_data: &'a [BlockData],
                       local_chunks: &'a LocalChunks) {
        ptr::write(&mut self.block_data, block_data);
        ptr::write(&mut self.local_chunks, local_chunks);

        ptr::write(&mut self.cpos, scalar(0));
        ptr::write(&mut self.iter, Region::new(scalar(0), scalar(0)).points());
    }

    pub fn reset(&mut self, cpos: V2) {
        self.cpos = cpos;
        self.iter = Region::new(scalar(0),
                                scalar::<V3>(CHUNK_SIZE) + V3::new(0, 1, 0)).points();
    }

    pub fn generate(&mut self,
                    buf: &mut [Vertex],
                    idx: &mut usize) -> bool {
        let local_bounds = Region::new(scalar(0), scalar(LOCAL_SIZE as i32));
        let chunk_bounds = Region::new(scalar(0), scalar(CHUNK_SIZE));

        while remaining_quads(buf, *idx) >= 4 {
            let iter_pos = match self.iter.next() {
                Some(p) => p,
                None => return false,
            };

            let adj_pos = iter_pos + V3::new(0, iter_pos.z, 0);
            let pos = self.cpos.extend(0) * scalar(CHUNK_SIZE) + adj_pos;
            let cpos = pos.div_floor(scalar(CHUNK_SIZE)) % scalar(LOCAL_SIZE);
            let offset = adj_pos % scalar(CHUNK_SIZE);

            let chunk_idx = local_bounds.index(cpos);
            let block_idx = chunk_bounds.index(offset);
            let block_id = self.local_chunks[chunk_idx][block_idx];
            let block = &self.block_data[block_id as usize];

            for side in 0 .. 4 {
                let tile = block.tile(side);
                if tile == 0 {
                    continue;
                }

                let s = tile % ATLAS_SIZE;
                let t = tile / ATLAS_SIZE;
                emit_quad(buf, idx, Vertex {
                    corner: (0, 0),
                    pos: (pos.x as u8,
                          pos.y as u8,
                          pos.z as u8),
                    side: side as u8,
                    tex_coord: (s as u8,
                                t as u8),
                });
            }
        }

        // Stopped because the buffer is full.
        true
    }
}
