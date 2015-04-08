#![crate_name = "graphics"]
#![no_std]

#![feature(no_std)]
#![feature(core)]

#![allow(unsigned_negation)]

#[macro_use] extern crate core;
#[cfg(asmjs)] #[macro_use] extern crate asmrt;
#[cfg(not(asmjs))] #[macro_use] extern crate std;
extern crate physics;

use core::prelude::*;

use physics::{TILE_BITS, CHUNK_BITS};
use physics::v3::{V3, scalar, Region};


#[cfg(asmjs)]
mod std {
    pub use core::cmp;
    pub use core::fmt;
    pub use core::marker;
}


const ATLAS_SIZE: u16 = 32;

const TILE_SIZE: u16 = 1 << TILE_BITS;
const CHUNK_SIZE: u16 = 1 << CHUNK_BITS;

const LOCAL_BITS: usize = 3;
const LOCAL_SIZE: u16 = 1 << LOCAL_BITS;


/// Tile numbers used to display a particular block.
#[derive(Copy)]
pub struct BlockDisplay {
    front: u16,
    back: u16,
    top: u16,
    bottom: u16,
}

impl BlockDisplay {
    pub fn tile(&self, side: usize) -> u16 {
        match side {
            0 => self.front,
            1 => self.back,
            2 => self.top,
            3 => self.bottom,
            _ => panic!("invalid side number"),
        }
    }
}

/// BlockDisplay for every block type known to the client.
pub type BlockData = [BlockDisplay; (ATLAS_SIZE * ATLAS_SIZE) as usize];


/// A chunk of terrain.  Each element is a block index.
pub type BlockChunk = [u16; 1 << (3 * CHUNK_BITS)];
/// BlockChunk for every chunk in the local region.
pub type LocalChunks = [BlockChunk; 1 << (2 * LOCAL_BITS)];

/// Copy a BlockChunk into the LocalChunks.
pub fn load_chunk(local: &mut LocalChunks,
                  chunk: &BlockChunk,
                  cx: u16,
                  cy: u16) {
    let cx = cx & (LOCAL_SIZE - 1);
    let cy = cy & (LOCAL_SIZE - 1);
    let idx = cy * LOCAL_SIZE + cx;

    local[idx as usize] = *chunk;
}


/// Vertex attributes for terrain.
#[allow(dead_code)]
#[derive(Copy)]
pub struct TerrainVertex {
    x: u8,
    y: u8,
    z: u8,
    _pad0: u8,
    s: u8,
    t: u8,
    _pad1: u8,
    _pad2: u8,
}

/// Maximum number of blocks that could be present in the output of generate_geometry.  The +1 is
/// because generate_geometry actually looks at `CHUNK_SIZE + 1` y-positions.
const GEOM_BLOCK_COUNT: u16 = CHUNK_SIZE * (CHUNK_SIZE + 1) * CHUNK_SIZE;
/// Number of vertices in each face.
const FACE_VERTS: usize = 6;
/// A list of TerrainVertex items to pass to OpenGL.  The upper bound is GEOM_BLOCK_COUNT blocks *
/// 4 faces (sides) per block * FACE_VERTS vertices per face, but usually not all elements will be
/// filled in.
pub type TerrainGeometryBuffer = [TerrainVertex; GEOM_BLOCK_COUNT as usize * 4 * FACE_VERTS];

/// Generate terrain geometry for a chunk.  The result contains all faces that might overlap the
/// z=0 plane of the indicated chunk.  This means it contains the +y,-z half of the named chunk and
/// the -y,+z half of the next chunk in the +y direction.
///
/// This can actually output blocks at `CHUNK_SIZE + 1` y-positions for a particular x,z.  Only
/// CHUNK_SIZE blocks are actually visible, but those CHUNK_SIZE blocks include a half-block at the
/// top and a half-block at the bottom.  This function doesn't bother splitting back/top from
/// front/bottom, and just outputs the whole block on each end.
///
///         /-------/ CHUNK_SIZE visible (diagonal slice)
///      *-+-+-+-+-+
///      |/| | | |/|
///      +-+-+-+-+-*
///      |---------| CHUNK_SIZE + 1 output
/// Corners marked * are output even though they aren't actually visible.
pub fn generate_geometry(local: &LocalChunks,
                         block_data: &BlockData,
                         geom: &mut TerrainGeometryBuffer,
                         cx: u16,
                         cy: u16) -> usize {

    let cx = cx & (LOCAL_SIZE - 1);
    let cy0 = cy & (LOCAL_SIZE - 1);
    let cy1 = (cy + 1) & (LOCAL_SIZE - 1);

    let bounds = Region::new(scalar(0), scalar(CHUNK_SIZE as i32));

    let mut out_idx = 0;

    const SIDE_OFFSETS: [((u8, u8), (u8, u8)); 4] = [
        // Front
        ((1, 1), (0, 1)),
        // Back
        ((0, 1), (0, 1)),
        // Top
        ((0, 1), (1, 0)),
        // Bottom
        ((0, 0), (1, 0)),
    ];

    const CORNERS: [(u8, u8); 4] = [
        (0, 0),
        (1, 0),
        (1, 1),
        (0, 1),
    ];

    const INDEXES: [usize; 6] = [0, 1, 2,  0, 2, 3];

    fn place(buf: &mut TerrainGeometryBuffer,
             base_idx: &mut usize,
             tile_id: u16,
             bx: i32,
             by: i32,
             bz: i32,
             side: usize) {
        let ((base_y, base_z), (step_y, step_z)) = SIDE_OFFSETS[side];

        let tile_s = (tile_id % ATLAS_SIZE) as u8;
        let tile_t = (tile_id / ATLAS_SIZE) as u8;

        for &idx in INDEXES.iter() {
            let (corner_u, corner_v) = CORNERS[idx];
            let vert = TerrainVertex {
                x: bx as u8 + corner_u,
                y: by as u8 + base_y + (step_y & corner_v),
                z: bz as u8 + base_z - (step_z & corner_v),
                _pad0: 0,
                s: tile_s + corner_u,
                t: tile_t + corner_v,
                _pad1: 0,
                _pad2: 0,
            };
            buf[*base_idx] = vert;
            *base_idx += 1;
        }
    }

    let chunk0 = &local[(cy0 * LOCAL_SIZE + cx) as usize];
    for z in range(0, CHUNK_SIZE as i32) {
        for y in range(z, CHUNK_SIZE as i32) {
            for x in range(0, CHUNK_SIZE as i32) {
                let block_id = chunk0[bounds.index(V3::new(x, y, z))] as usize;
                for side in range(0, 4) {
                    let tile_id = block_data[block_id].tile(side);
                    if tile_id == 0 {
                        continue;
                    }
                    place(geom, &mut out_idx, tile_id, x, y, z, side);
                }
            }
        }
    }

    let chunk1 = &local[(cy1 * LOCAL_SIZE + cx) as usize];
    for z in range(0, CHUNK_SIZE as i32) {
        for y in range(0, z + 1) {  // NB: 0..z+1 instead of z..SIZE
            for x in range(0, CHUNK_SIZE as i32) {
                let block_id = chunk1[bounds.index(V3::new(x, y, z))] as usize;
                for side in range(0, 4) {
                    let tile_id = block_data[block_id].tile(side);
                    if tile_id == 0 {
                        continue;
                    }
                    place(geom, &mut out_idx, tile_id, x, y + 16, z, side);  // NB: +16
                }
            }
        }
    }

    out_idx
}
