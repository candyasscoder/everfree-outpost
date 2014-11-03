#![crate_name = "graphics"]
#![no_std]
#![feature(globs, phase)]
#![feature(overloaded_calls, unboxed_closures)]
#![feature(macro_rules)]

#[phase(plugin, link)] extern crate core;
#[cfg(asmjs)]
#[phase(plugin, link)] extern crate asmrt;
#[cfg(not(asmjs))]
#[phase(plugin, link)] extern crate std;
extern crate physics;

use core::prelude::*;
use core::cmp;
use core::fmt;
use core::iter::range_inclusive;

use physics::v3::V3;
use physics::{TILE_BITS, CHUNK_BITS};


#[cfg(asmjs)]
mod std {
    pub use core::cmp;
    pub use core::fmt;
}


const ATLAS_SIZE: u16 = 32;

const TILE_SIZE: u16 = 1 << TILE_BITS;
const CHUNK_SIZE: u16 = 1 << CHUNK_BITS;

const LOCAL_BITS: uint = 3;
const LOCAL_SIZE: u16 = 1 << LOCAL_BITS;

pub struct BlockDisplay {
    front: u16,
    back: u16,
    top: u16,
    bottom: u16,
}

pub type BlockData = [BlockDisplay, ..(ATLAS_SIZE * ATLAS_SIZE) as uint];

pub type ChunkData = [u16, ..1 << (3 * CHUNK_BITS)];
pub type LocalData = [ChunkData, ..1 << (2 * LOCAL_BITS)];


/*
pub enum Surface {
    Empty,
    Output,
    TileAtlas,
    RenderCache(u8),
    ChunkCache(u8),
}
*/
pub type Surface = u16;
#[allow(non_upper_case_globals)]
pub const Empty: Surface = 0;
#[allow(non_upper_case_globals)]
pub const Output: Surface = 1;
#[allow(non_upper_case_globals)]
pub const TileAtlas: Surface = 2;
#[allow(non_snake_case)]
pub fn RenderCache(i: u8) -> Surface { 8 + i as u16 }
#[allow(non_snake_case)]
pub fn ChunkCache(i: u8) -> Surface { 64 + i as u16 }
#[allow(non_snake_case)]
pub fn SpriteImage(i: u16) -> Surface { 128 + i as u16 }


enum Side {
    Front,
    Back,
    Top,
    Bottom,
}

pub struct RenderContext<'a> {
    pub block_data: &'a BlockData,
    pub local_data: &'a LocalData,
}

impl<'a> RenderContext<'a> {
    fn get_tile(&self, x: u16, y: u16, z: u16, side: Side) -> u16 {
        if z >= CHUNK_SIZE {
            return 0;
        }

        let i = y / CHUNK_SIZE % LOCAL_SIZE;
        let j = x / CHUNK_SIZE % LOCAL_SIZE;
        let chunk_idx = i * LOCAL_SIZE + j;

        let tx = x % CHUNK_SIZE;
        let ty = y % CHUNK_SIZE;
        let tile_idx = (z * CHUNK_SIZE + ty) * CHUNK_SIZE + tx;

        let block_id = self.local_data[chunk_idx as uint][tile_idx as uint];
        let block = &self.block_data[block_id as uint];
        match side {
            Front => block.front,
            Back => block.back,
            Top => block.top,
            Bottom => block.bottom,
        }
    }
}


struct XvItem {
    tiles: [u16, ..(CHUNK_SIZE * 4) as uint],
}

pub type XvData = [XvItem, ..1 << (2 * (CHUNK_BITS + LOCAL_BITS))];

unsafe fn _static_assert() {
    use core::mem;
    let _ = mem::transmute::<XvItem, [u16, ..(CHUNK_SIZE * 4) as uint]>(mem::zeroed());
}

pub fn update_xv(xv: &mut XvData, blocks: &BlockData, chunk: &ChunkData, i: u8, j: u8) {
    let base_v = i as u16 * CHUNK_SIZE;
    let base_x = j as u16 * CHUNK_SIZE;

    fn get_chunk_idx(x: u16, u: u16, v: u16) -> uint {
        let y = (u + v) / 2;
        let z = (u - v) / 2;
        ((z * CHUNK_SIZE + y) * CHUNK_SIZE + x) as uint
    }

    fn get_xv_idx(x: u16, v: u16) -> uint {
        (v as uint << (CHUNK_BITS + LOCAL_BITS)) + x as uint
    }

    let mut copy = |&mut: lower, local_x, local_u: u16, local_v, chunk_x, chunk_u, chunk_v| {
        let chunk_idx = get_chunk_idx(chunk_x, chunk_u, chunk_v);
        let xv_idx = get_xv_idx(local_x, local_v);
        if lower {
            xv[xv_idx].tiles[local_u as uint * 2 + 0] = blocks[chunk[chunk_idx] as uint].bottom;
            xv[xv_idx].tiles[local_u as uint * 2 + 1] = blocks[chunk[chunk_idx] as uint].front;
        } else {
            xv[xv_idx].tiles[local_u as uint * 2 + 0] = blocks[chunk[chunk_idx] as uint].back;
            xv[xv_idx].tiles[local_u as uint * 2 + 1] = blocks[chunk[chunk_idx] as uint].top;
        }
    };

    for chunk_v_base in range(0, CHUNK_SIZE) {
        for chunk_x in range(0, CHUNK_SIZE) {
            let local_x = (base_x + chunk_x) % (CHUNK_SIZE * LOCAL_SIZE);
            let local_v0 = (base_v + chunk_v_base) % (CHUNK_SIZE * LOCAL_SIZE);
            let local_v1 = (base_v - chunk_v_base - 1) % (CHUNK_SIZE * LOCAL_SIZE);

            let num_blocks = 2 * (CHUNK_SIZE - chunk_v_base) - 1;
            for offset_u in range(0, num_blocks) {
                let lower = offset_u % 2 == 0;
                let local_u0 = offset_u;
                let local_u1 = 2 * CHUNK_SIZE - num_blocks + offset_u;

                let chunk_x = chunk_x;
                let chunk_u = chunk_v_base + offset_u;
                let chunk_v = chunk_v_base + (!lower) as u16;

                copy(lower, local_x, local_u0, local_v0, chunk_x, chunk_u, chunk_v);
                copy(!lower, local_x, local_u1, local_v1, chunk_x, chunk_u, -chunk_v);
            }
        }
    }
}


pub struct Sprite {
    id: u16,
    ref_x: u16,
    ref_y: u16,
    ref_z: u16,
    width: u16,
    height: u16,
    anchor_x: u16,
    anchor_y: u16,
}

impl Sprite {
    fn screen_pos(&self) -> (u16, u16) {
        (self.ref_x - self.anchor_x,
         self.ref_y - self.ref_z - self.anchor_y)
    }

    fn ref_uv(&self) -> (u16, u16) {
        (self.ref_y + self.ref_z,
         self.ref_y - self.ref_z)
    }
}


pub fn render(xv: &XvData,
              x: u16,
              y: u16,
              width: u16,
              height: u16,
              sprites: &mut [Sprite],
              callback: |Surface, u16, u16, Surface, u16, u16, u16, u16|) {
    let chunk_px = CHUNK_SIZE * TILE_SIZE;

    let min_i = y / chunk_px;
    let max_i = (y + height + chunk_px - 1) / chunk_px;
    let min_j = x / chunk_px;
    let max_j = (x + width + chunk_px - 1) / chunk_px;

    for raw_i in range(min_i, max_i) {
        for raw_j in range(min_j, max_j) {
            let i = raw_i % LOCAL_SIZE;
            let j = raw_j % LOCAL_SIZE;
            let idx = i * LOCAL_SIZE + j;

            callback(ChunkCache(idx as u8), 0, 0,
                     Output, raw_j * chunk_px, (raw_i - 1) * chunk_px,
                     chunk_px, chunk_px * 2);
        }
    }

    quicksort(sprites, SpriteUV);
    render_sprites(xv, x, y, width, height, sprites, callback);
}

const LEVEL_BUFFER_SIZE: uint = 1024;

fn render_sprites(xv: &XvData,
                  x: u16,
                  y: u16,
                  width: u16,
                  height: u16,
                  sprites: &mut [Sprite],
                  mut callback: |Surface, u16, u16, Surface, u16, u16, u16, u16|) {
    let screen_min_row = y / TILE_SIZE;
    let screen_min_col = x / TILE_SIZE;
    let screen_max_row = (y + height + TILE_SIZE - 1) / TILE_SIZE;
    let screen_max_col = (x + width + TILE_SIZE - 1) / TILE_SIZE;
    let screen_rows = screen_max_row - screen_min_row;
    let screen_cols = screen_max_col - screen_min_col;

    if screen_rows as uint * screen_cols as uint > LEVEL_BUFFER_SIZE {
        let split =
            if width > height {
                let left = width / 2;
                ((x, y, left, height),
                 (x + left, y, width - left, height))
            } else {
                let top = height / 2;
                ((x, y, width, top),
                 (x, y + top, width, height - top))
            };
        let ((x0, y0, w0, h0), (x1, y1, w1, h1)) = split;

        // TODO: set clip?
        render_sprites(xv, x0, y0, w0, h0, sprites,
                       |src, sx, sy, dst, dx, dy, w, h|
                       callback(src, sx, sy, dst, dx, dy, w, h));
        render_sprites(xv, x1, y1, w1, h1, sprites,
                       |src, sx, sy, dst, dx, dy, w, h|
                       callback(src, sx, sy, dst, dx, dy, w, h));
        return;
    }

    let mut draw_level = [0_u8, ..LEVEL_BUFFER_SIZE];
    let get_index = |&: row: u16, col: u16| {
        let i = row - screen_min_row;
        let j = col - screen_min_col;
        (i * screen_cols + j) as uint
    };

    let mut draw_stack = |&mut: row: u16, col: u16, min_u: u8, max_u: u8,
                          callback: &mut |Surface, u16, u16, Surface, u16, u16, u16, u16|| {
        let x = col % (CHUNK_SIZE * LOCAL_SIZE);
        let v = row % (CHUNK_SIZE * LOCAL_SIZE);
        let xv_idx = v * CHUNK_SIZE * LOCAL_SIZE + x;

        for u in range(min_u, max_u) {
            let t = xv[xv_idx as uint].tiles[u as uint];
            if t != 0 {
                (*callback)(TileAtlas,
                            (t % ATLAS_SIZE) * TILE_SIZE,
                            (t / ATLAS_SIZE) * TILE_SIZE,
                            Output,
                            col * TILE_SIZE,
                            row * TILE_SIZE,
                            TILE_SIZE,
                            TILE_SIZE);
            }
        }
    };

    for sprite in sprites.iter() {
        let (screen_x, screen_y) = sprite.screen_pos();
        let min_row = screen_y / TILE_SIZE;
        let min_col = screen_x / TILE_SIZE;
        let max_row = (screen_y + sprite.height + TILE_SIZE - 1) / TILE_SIZE;
        let max_col = (screen_x + sprite.width + TILE_SIZE - 1) / TILE_SIZE;

        for row in range(cmp::max(min_row, screen_min_row),
                         cmp::min(max_row, screen_max_row)) {
            // Number of surfaces in each x,v position on this row that are entirely behind or
            // entirely below the sprite.  These counts use the same units as XvData.tiles indices,
            // so the front of one tile and the back of the adjacent tile are counted separately.
            let behind = 4 * (sprite.ref_y / TILE_SIZE - row) - 1;
            let below = 4 * (sprite.ref_z / TILE_SIZE) + 1;
            let limit = cmp::max(0, cmp::max(behind as i16, below as i16)) as u8;

            for col in range(cmp::max(min_col, screen_min_col),
                             cmp::min(max_col, screen_max_col)) {
                let start = draw_level[get_index(row, col)];
                if start == 0 {
                    callback(Empty, 0, 0,
                             Output, col * TILE_SIZE, row * TILE_SIZE,
                             TILE_SIZE, TILE_SIZE);
                }

                draw_stack(row, col, start, limit, &mut callback);
                draw_level[get_index(row, col)] = limit;
            }
        }

        callback(SpriteImage(sprite.id), 0, 0,
                 Output, screen_x, screen_y,
                 sprite.width, sprite.height);
        callback(Empty, 0, 0,
                 Output, sprite.ref_x - 4, sprite.ref_y - sprite.ref_z - 4,
                 8, 8);
    }

    for row in range(screen_min_row, screen_max_row) {
        for col in range(screen_min_col, screen_max_col) {
            let start = draw_level[get_index(row, col)];
            if start != 0 {
                draw_stack(row, col, start, (CHUNK_SIZE * 4) as u8, &mut callback);
            }
        }
    }
}


pub trait Compare<T> {
    fn is_less(&self, a: &T, b: &T) -> bool;
}

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

    fn partition<T, C>(xs: &mut [T], comp: C) -> uint
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
