#![crate_name = "graphics"]
#![no_std]

#![feature(unboxed_closures)]

#![allow(unsigned_negation)]
#![allow(unstable)] // many parts of libcore are unstable as of Rust 1.0

#[macro_use] extern crate core;
#[cfg(asmjs)] #[macro_use] extern crate asmrt;
#[cfg(not(asmjs))] #[macro_use] extern crate std;
extern crate physics;

use core::prelude::*;
use core::cell::Cell;
use core::cmp;

use physics::{TILE_BITS, CHUNK_BITS};


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

#[derive(Copy)]
pub struct BlockDisplay {
    front: u16,
    back: u16,
    top: u16,
    bottom: u16,
}

pub type BlockData = [BlockDisplay; (ATLAS_SIZE * ATLAS_SIZE) as usize];

pub type ChunkData = [u16; 1 << (3 * CHUNK_BITS)];
pub type LocalData = [ChunkData; 1 << (2 * LOCAL_BITS)];


type XvOffsets = [u8; (CHUNK_SIZE * 4) as usize];
type XvTiles = [u16; (CHUNK_SIZE * 4) as usize];

struct XvChunk {
    bases: [u16; 1 << (2 * CHUNK_BITS)],
    offsets: [XvOffsets; 1 << (2 * CHUNK_BITS)],
    tiles: [XvTiles; 1 << (2 * CHUNK_BITS)],
}

pub struct XvData {
    chunks: [XvChunk; 1 << (2 * LOCAL_BITS)],
}

pub fn update_xv(xv: &mut XvData, blocks: &BlockData, chunk: &ChunkData, i: u8, j: u8) {
    let i = i as usize;
    let j = j as usize;
    let chunk_idx0 = (i << LOCAL_BITS) + j;
    let i1 = (i + LOCAL_SIZE as usize - 1) % LOCAL_SIZE as usize;
    let chunk_idx1 = (i1 << LOCAL_BITS) + j;

    let mut copy = |&mut: x: u16, u: u16, v: u16,
                          chunk_idx: usize,
                          cx: u16, cu: u16, cv: u16,
                          upper: bool| {
        let tiles_idx = ((v << CHUNK_BITS) + x) as usize;
        let stack = &mut xv.chunks[chunk_idx].tiles[tiles_idx];

        let cy = (cu + cv) / 2;
        let cz = (cu - cv) / 2;
        let tile = chunk[((cz * CHUNK_SIZE + cy) * CHUNK_SIZE + cx) as usize];
        let block = &blocks[tile as usize];

        if upper {
            stack[2 * u as usize + 0] = block.back;
            stack[2 * u as usize + 1] = block.top;
        } else {
            stack[2 * u as usize + 0] = block.bottom;
            stack[2 * u as usize + 1] = block.front;
        }
    };

    for v in range(0, CHUNK_SIZE) {
        let base_cu = CHUNK_SIZE - v - 1;
        let base_cv = -base_cu;
        for x in range(0, CHUNK_SIZE) {
            for k in range(0, 2 * v + 1) {
                let odd = k % 2 == 1;

                let u = k + 2 * (CHUNK_SIZE - v) - 1;

                let cx = x;
                let cu = base_cu + k;
                let cv = base_cv - (odd as u16);

                copy(x, u, v, chunk_idx1, cx, cu, cv, !odd);
            }
        }
    }

    for v in range(0, CHUNK_SIZE) {
        let base_cu = v;
        let base_cv = v;
        for x in range(0, CHUNK_SIZE) {
            for k in range(0, 2 * (CHUNK_SIZE - v) - 1) {
                let odd = k % 2 == 1;

                let u = k;

                let cx = x;
                let cu = base_cu + k;
                let cv = base_cv + (odd as u16);

                copy(x, u, v, chunk_idx0, cx, cu, cv, odd);
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Copy)]
pub struct VertexData {
    x: u8,
    y: u8,
    s: u8,
    t: u8,
}
// Each chunk has CHUNK_SIZE^3 blocks, each block has 4 faces, each face has 6 vertices.
const FACE_VERTS: usize = 6;
pub type GeometryBuffer = [VertexData; (4 * FACE_VERTS) << (3 * CHUNK_BITS)];


pub fn generate_geometry(xv: &mut XvData,
                         geom: &mut GeometryBuffer,
                         i: u8, j: u8) -> usize {
    let pos = Cell::new(0);

    let mut push = |&mut: x, y, s, t| {
        geom[pos.get()] = VertexData { x: x, y: y, s: s, t: t };
        pos.set(pos.get() + 1);
    };
    let mut push_face = |&mut: x: u16, y: u16, tile: u16| {
        let (x, y) = (x as u8, y as u8);
        let s = (tile % ATLAS_SIZE) as u8;
        let t = (tile / ATLAS_SIZE) as u8;

        push(x,     y,     s,     t    );
        push(x,     y + 1, s,     t + 1);
        push(x + 1, y + 1, s + 1, t + 1);

        push(x,     y,     s,     t    );
        push(x + 1, y + 1, s + 1, t + 1);
        push(x + 1, y,     s + 1, t    );

    };

    let chunk = &mut xv.chunks[((i << LOCAL_BITS) + j) as usize];

    for v in range(0, CHUNK_SIZE) {
        for x in range(0, CHUNK_SIZE) {
            let idx = (v * CHUNK_SIZE + x) as usize;
            let base = pos.get() / FACE_VERTS;
            chunk.bases[idx] = base as u16;
            for u in range(0, 4 * CHUNK_SIZE) {
                let tile = chunk.tiles[idx][u as usize];
                if tile != 0 {
                    push_face(x, v, tile);
                }
                let offset = (pos.get() / FACE_VERTS) - base as usize;
                chunk.offsets[idx][u as usize] = offset as u8;
            }
        }
    }

    pos.get()
}


#[derive(PartialEq, Eq, Show)]
enum SpriteStatus {
    // Sprite does not overlap the chunk.
    Outside,
    // Sprite will be drawn above all terrain in the chunk.
    Above,
    // Sprite will be drawn between some terrain layers in the chunk.
    Between,
}

#[derive(Show)]
pub struct Sprite {
    id: u16,
    ref_x: u16,
    ref_y: u16,
    ref_z: u16,
    width: u8,
    height: u8,
    anchor_x: u8,
    anchor_y: u8,
    status: SpriteStatus,
}

impl Sprite {
    fn screen_pos(&self) -> (u16, u16) {
        (self.ref_x - self.anchor_x as u16,
         self.ref_y - self.ref_z - self.anchor_y as u16)
    }

    fn ref_uv(&self) -> (u16, u16) {
        (self.ref_y + self.ref_z,
         self.ref_y - self.ref_z)
    }

    fn clipped_bounds(&self, cx: u16, cy: u16) -> (u16, u16, u16, u16) {
        let (screen_x, screen_y) = self.screen_pos();

        let chunk_px = CHUNK_SIZE * TILE_SIZE;
        let base_x0 = cx * chunk_px;
        let base_x1 = base_x0 + chunk_px;
        let base_y0 = cy * chunk_px;
        let base_y1 = base_y0 + chunk_px;

        fn clamp(x: u16, min: u16, max: u16) -> u16 {
            if x < min { min }
            else if x > max { max }
            else { x }
        }

        let x0 = clamp(screen_x, base_x0, base_x1) - base_x0;
        let x1 = clamp(screen_x + self.width as u16, base_x0, base_x1) - base_x0;
        let y0 = clamp(screen_y, base_y0, base_y1) - base_y0;
        let y1 = clamp(screen_y + self.height as u16, base_y0, base_y1) - base_y0;

        (x0, x1, y0, y1)
    }

    fn clipped_tile_bounds(&self, cx: u16, cy: u16) -> (u16, u16, u16, u16) {
        let (x0, x1, y0, y1) = self.clipped_bounds(cx, cy);

        let tx0 = x0 / TILE_SIZE;
        let tx1 = (x1 + TILE_SIZE - 1) / TILE_SIZE;
        let ty0 = y0 / TILE_SIZE;
        let ty1 = (y1 + TILE_SIZE - 1) / TILE_SIZE;

        (tx0, tx1, ty0, ty1)
    }

    fn u_limit(&self, row: u16) -> u8 {
        // Number of surfaces in each x,v position on this row that are entirely behind or entirely
        // below the sprite.  These counts use the same units as XvData.tiles indices, so the front
        // of one tile and the back of the adjacent tile are counted separately.
        let behind = 4 * (self.ref_y / TILE_SIZE - row) - 1;
        let below = 4 * (self.ref_z / TILE_SIZE) + 1;
        let limit = cmp::max(0, cmp::max(behind as i16, below as i16));
        limit as u8
    }
}


pub fn render<FT, FS>(xv: &XvData,
                      x: u16,
                      y: u16,
                      width: u16,
                      height: u16,
                      sprites: &mut [Sprite],
                      mut draw_terrain: FT,
                      mut draw_sprite: FS)
        where FT: FnMut(u16, u16, u16, u16),
              FS: FnMut(u16, u16, u16, u16, u16) {
    quicksort(sprites, SpriteUV);

    let chunk_px = CHUNK_SIZE * TILE_SIZE;

    let cx0 = x / chunk_px;
    let cx1 = (x + width + chunk_px - 1) / chunk_px;
    let cy0 = y / chunk_px;
    let cy1 = (y + height + chunk_px - 1) / chunk_px;

    for cy in range(cy0, cy1) {
        for cx in range(cx0, cx1) {
            let i = cy % LOCAL_SIZE;
            let j = cx % LOCAL_SIZE;
            let idx = (i * LOCAL_SIZE + j) as usize;

            let chunk = &xv.chunks[idx];
            render_chunk(chunk, cx, cy, sprites,
                         |a, b, c, d| draw_terrain(a, b, c, d),
                         |a, b, c, d, e| draw_sprite(a, b, c, d, e));
        }
    }
}

fn render_chunk<FT, FS>(chunk: &XvChunk,
                        cx: u16, cy: u16,
                        sprites: &mut [Sprite],
                        mut draw_terrain: FT,
                        mut draw_sprite: FS)
        where FT: FnMut(u16, u16, u16, u16),
              FS: FnMut(u16, u16, u16, u16, u16) {
    // Pass #1
    //
    // Examine each sprite.  For each sprite, determine whether it is Outside, Above, or Between
    // the terrain layers.  Also mark each column in which terrain covers a sprite, as these
    // columns need to be drawn incrementally.
    let mut depth = [-1_u8; 1 << (2 * CHUNK_BITS)];

    let max_idx = (CHUNK_SIZE * CHUNK_SIZE - 1) as usize;
    let col_max_idx = (4 * CHUNK_SIZE - 1) as usize;

    for sprite in sprites.iter_mut() {
        let (tx0, tx1, ty0, ty1) = sprite.clipped_tile_bounds(cx, cy);
        let inside = tx0 < tx1 && ty0 < ty1;
        let mut below = false;
        for ty in range(ty0, ty1) {
            let limit = sprite.u_limit(cy * CHUNK_SIZE + ty);
            for tx in range(tx0, tx1) {
                let idx = (ty * CHUNK_SIZE + tx) as usize;
                // Offset of the first face above the sprite in this column.  We look at index
                // `limit - 1` is the first face *after* the one at u=limit, and we want the last
                // one *before* u=limit.  This never underflows because limit is always >= 1
                // (assuming ref_z >= 0).
                let above = chunk.offsets[idx][limit as usize - 1];
                // Offset of the last face in this column.
                let col_max = chunk.offsets[idx][col_max_idx];

                if above < col_max {
                    depth[idx] = 0;
                    below = true;
                }
            }
        }

        sprite.status =
            if !inside {
                SpriteStatus::Outside
            } else if !below {
                SpriteStatus::Above
            } else {
                SpriteStatus::Between
            };
    }

    // Pass #2
    //
    // Examine each column.  Collect runs of columns tha contain only Above sprites (or no sprites)
    // and draw each run.
    let mut last = 0;
    for idx in range(0, depth.len()) {
        if depth[idx] == -1 {
            continue;
        }

        let cur = chunk.bases[idx];
        if cur > last {
            draw_terrain(cx, cy, last, cur);
        }
        last = cur + chunk.offsets[idx][col_max_idx] as u16;
    }

    let cur = chunk.bases[max_idx] + chunk.offsets[max_idx][col_max_idx] as u16;
    if cur > last {
        draw_terrain(cx, cy, last, cur);
    }

    // Pass #3 [TODO]
    //
    // Examine each sprite.  Draw all partial columns below the sprite, then draw the sprite
    // itself.
    for sprite in sprites.iter() {
        if sprite.status == SpriteStatus::Outside {
            continue;
        }

        if sprite.status == SpriteStatus::Between {
            let (tx0, tx1, ty0, ty1) = sprite.clipped_tile_bounds(cx, cy);
            for ty in range(ty0, ty1) {
                let limit = sprite.u_limit(cy * CHUNK_SIZE + ty);
                for tx in range(tx0, tx1) {
                    let idx = (ty * CHUNK_SIZE + tx) as usize;
                    let base = chunk.bases[idx];
                    let start_off = depth[idx];
                    let end_off = chunk.offsets[idx][limit as usize - 1];
                    if start_off < end_off {
                        draw_terrain(cx, cy,
                                     base + start_off as u16,
                                     base + end_off as u16);
                    }
                    depth[idx] = end_off;
                }
            }
        }

        let (x0, x1, y0, y1) = sprite.clipped_bounds(cx, cy);
        let base_x = cx * CHUNK_SIZE * TILE_SIZE;
        let base_y = cy * CHUNK_SIZE * TILE_SIZE;
        draw_sprite(sprite.id,
                    x0 + base_x,
                    y0 + base_y,
                    x1 - x0,
                    y1 - y0);
    }

    // Pass #4 [TODO]
    //
    // Examine each column.  If the column has been partially drawn, draw the topmost part of it.
    for ty in range(0, CHUNK_SIZE) {
        for tx in range(0, CHUNK_SIZE) {
            let idx = (ty * CHUNK_SIZE + tx) as usize;
            if depth[idx] == -1 {
                continue;
            }

            let base = chunk.bases[idx];
            let start_off = depth[idx];
            let end_off = chunk.offsets[idx][col_max_idx];
            if start_off < end_off {
                draw_terrain(cx, cy,
                             base + start_off as u16,
                             base + end_off as u16);
            }
        }
    }
}


pub trait Compare<T> {
    fn is_less(&self, a: &T, b: &T) -> bool;
}

#[derive(Copy)]
struct SpriteUV;
impl Compare<Sprite> for SpriteUV {
    fn is_less(&self, a: &Sprite, b: &Sprite) -> bool {
        let (au, av) = a.ref_uv();
        let (bu, bv) = b.ref_uv();
        if au != bu {
            au < bu
        } else {
            av < bv
        }
    }
}

fn quicksort<T, C>(xs: &mut [T], comp: C)
        where C: Compare<T> + Copy {
    // Based on pseudocode from wikipedia: https://en.wikipedia.org/wiki/Quicksort
    if xs.len() <= 1 {
        return;
    }

    let pivot = partition(xs, comp);
    quicksort(xs.slice_to_mut(pivot), comp);
    quicksort(xs.slice_from_mut(pivot + 1), comp);

    fn partition<T, C>(xs: &mut [T], comp: C) -> usize
            where C: Compare<T> + Copy {
        // Always choose rightmost element as the pivot.
        let pivot = xs.len() - 1;
        let mut store_index = 0;
        for i in range(0, xs.len() - 1) {
            if comp.is_less(&xs[i], &xs[pivot]) {
                xs.swap(i, store_index);
                store_index += 1;
            }
        }
        xs.swap(store_index, pivot);
        store_index
    }
}
