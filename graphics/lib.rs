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
use physics::{CHUNK_SIZE, CHUNK_BITS, CHUNK_MASK};


#[cfg(asmjs)]
mod std {
    pub use core::cmp;
    pub use core::fmt;
}



pub const HAS_TOP: u8       = 0x01;
pub const HAS_BOTTOM: u8    = 0x02;
pub const HAS_FRONT: u8     = 0x04;
pub const HAS_BACK: u8      = 0x08;
pub const OPAQUE_TOP: u8    = 0x10;
pub const OPAQUE_BOTTOM: u8 = 0x20;
pub const OPAQUE_FRONT: u8  = 0x40;
pub const OPAQUE_BACK: u8   = 0x80;


#[deriving(Show)]
pub struct Layer {
    x_min: u8,
    y_min: u8,
    z_min: u8,
    x_max: u8,
    y_max: u8,
    z_max: u8,
    pos: u16,
}

impl Layer {
    pub fn new(min: V3, max: V3) -> Layer {
        Layer {
            x_min: min.x as u8,
            y_min: min.y as u8,
            z_min: min.z as u8,
            x_max: max.x as u8,
            y_max: max.y as u8,
            z_max: max.z as u8,
            pos: 0,
        }
    }

    pub fn expand(&mut self, min: V3, max: V3) {
        self.x_min = cmp::min(self.x_min, min.x as u8);
        self.y_min = cmp::min(self.y_min, min.y as u8);
        self.z_min = cmp::min(self.z_min, min.z as u8);

        self.x_max = cmp::max(self.x_max, max.x as u8);
        self.y_max = cmp::max(self.y_max, max.y as u8);
        self.z_max = cmp::max(self.z_max, max.z as u8);
    }

    pub fn width(&self) -> u8 {
        self.x_max - self.x_min
    }

    pub fn height(&self) -> u8 {
        let y_size = self.y_max - self.y_min;
        let z_size = self.z_max - self.z_min;
        if y_size != 0 {
            y_size
        } else {
            z_size
        }
    }
}

// Maximum possible number of horizontal layers occurs when all CHUNK_SIZE + 1 planes are
// checkerboard tiled.  Checkerboard tiling produces CHUNK_SIZE * CHUNK_SIZE / 2 layers, since half
// the tiles are filled, and every layer is a single tile.  Finally, multiply by 2, since the same
// can occur along the vertical plane.
pub const MAX_LAYERS: i32 = (CHUNK_SIZE + 1) * (CHUNK_SIZE * CHUNK_SIZE / 2) * 2;

pub struct BakerState<'a> {
    flags: &'a [u8, ..1 << (3 * CHUNK_BITS)],
    layers: &'a mut [Layer, ..MAX_LAYERS as uint],
    next_layer: i32,
}

impl<'a> BakerState<'a> {
    pub fn new(flags: &'a [u8, ..1 << (3 * CHUNK_BITS)],
               layers: &'a mut [Layer, ..MAX_LAYERS as uint]) -> BakerState<'a> {
        BakerState {
            flags: flags,
            layers: layers,
            next_layer: 0,
        }
    }

    pub fn bake(&mut self) -> (i32, i32) {
        self.bake_z();
        self.bake_y();
        let layers = self.next_layer;
        let pages = pack_layers(self.layers.slice_to_mut(self.next_layer as uint));
        (layers, pages)
    }

    pub fn bake_z(&mut self) {
        for z in range_inclusive(0, CHUNK_SIZE) {
            let mut seen = [false, ..1 << (2 * CHUNK_BITS)];

            for y in range(0, CHUNK_SIZE) {
                for x in range(0, CHUNK_SIZE) {
                    if seen[(y * CHUNK_SIZE + x) as uint] ||
                       !self.has_horiz_plane(V3::new(x, y, z)) {
                        continue;
                    }

                    let mut layer = Layer::new(V3::new(-1, -1, z), V3::new(0, 0, z));
                    let mut count = 0u;

                    flood(&mut seen, y, x,
                          |&: y, x| { self.has_horiz_plane(V3::new(x, y, z)) },
                          |&mut: y, x| {
                              count += 1;
                              layer.expand(V3::new(x, y, z),
                                           V3::new(x + 1, y + 1, z));
                          });
                    if count > 0 {
                        self.add_layer(layer);
                    }
                }
            }
        }
    }

    fn bake_y(&mut self) {
        for y in range_inclusive(0, CHUNK_SIZE) {
            let mut seen = [false, ..1 << (2 * CHUNK_BITS)];

            for z in range(0, CHUNK_SIZE) {
                for x in range(0, CHUNK_SIZE) {
                    if seen[(z * CHUNK_SIZE + x) as uint] ||
                       !self.has_vert_plane(V3::new(x, y, z)) {
                        continue;
                    }

                    let mut layer = Layer::new(V3::new(-1, y, -1), V3::new(0, y, 0));
                    let mut count = 0u;

                    flood(&mut seen, z, x,
                          |&: z, x| { self.has_vert_plane(V3::new(x, y, z)) },
                          |&mut: z, x| {
                              count += 1;
                              layer.expand(V3::new(x, y, z),
                                           V3::new(x + 1, y, z + 1));
                          });
                    if count > 0 {
                        self.add_layer(layer);
                    }
                }
            }
        }
    }

    fn get_flags(&self, pos: V3) -> u8 {
        let index = (pos.z * CHUNK_SIZE + pos.y) * CHUNK_SIZE + pos.x;
        self.flags[index as uint]
    }

    fn has_flag(&self, pos: V3, flags: u8) -> bool {
        self.get_flags(pos) & flags != 0
    }

    fn has_horiz_plane(&self, pos: V3) -> bool {
        let below = pos - V3::new(0, 0, 1);
        pos.z > 0 && self.has_flag(below, HAS_TOP) ||
        pos.z < CHUNK_SIZE && self.has_flag(pos, HAS_BOTTOM)
    }

    fn has_vert_plane(&self, pos: V3) -> bool {
        let behind = pos - V3::new(0, 1, 0);
        pos.y > 0 && self.has_flag(behind, HAS_FRONT) ||
        pos.y < CHUNK_SIZE && self.has_flag(pos, HAS_BACK)
    }

    fn add_layer(&mut self, layer: Layer) {
        self.layers[self.next_layer as uint] = layer;
        self.next_layer += 1;
    }
}

#[inline]
fn flood<F1, F2>(seen: &mut [bool, ..1 << (2 * CHUNK_BITS)],
                 i: i32, j: i32,
                 is_valid: F1,
                 mut process: F2)
        where F1: Fn(i32, i32) -> bool,
              F2: FnMut(i32, i32) {
    let mut stack = [0, ..1 << (2 * CHUNK_BITS)];
    let mut top;

    let idx = i * CHUNK_SIZE + j;
    if seen[idx as uint] {
        return;
    }
    seen[idx as uint] = true;
    stack[0] = idx as u8;
    top = 1;

    while top > 0 {
        top -= 1;
        let idx = stack[top];

        let j = idx as i32 & CHUNK_MASK;
        let i = (idx >> CHUNK_BITS) as i32 & CHUNK_MASK;

        if !is_valid(i, j) {
            continue;
        }

        process(i, j);

        if i > 0 {
            maybe_push(&mut stack, &mut top, seen, i - 1, j);
        }
        if i < CHUNK_SIZE - 1 {
            maybe_push(&mut stack, &mut top, seen, i + 1, j);
        }
        if j > 0 {
            maybe_push(&mut stack, &mut top, seen, i, j - 1);
        }
        if j < CHUNK_SIZE - 1 {
            maybe_push(&mut stack, &mut top, seen, i, j + 1);
        }
    }

    #[inline]
    fn maybe_push(stack: &mut [u8, ..1 << (2 * CHUNK_BITS)],
                  top: &mut uint,
                  seen: &mut [bool, ..1 << (2 * CHUNK_BITS)],
                  i: i32, j: i32) {
        let idx = i * CHUNK_SIZE + j;
        if seen[idx as uint] {
            return;
        }
        seen[idx as uint] = true;

        stack[*top] = idx as u8;
        *top += 1;
    }
}


/// Pack layers into pages, so that the baked chunk can use a few large offscreen buffers instead
/// of many small ones.  Sets `pos` for each layer to indicate its page and position on the page.
/// Layers do not overlap each other or the page boundaries.
pub fn pack_layers(layers: &mut [Layer]) -> i32 {
    quicksort(layers, LayerWH);

    // Dimensions of a page.
    const WIDTH: uint = 16;
    const HEIGHT: uint = 32;

    let mut skyline = [0, ..HEIGHT];

    let mut large = layers.len() - 1;
    let mut small = 0;

    let mut page = 0;

    'a: loop {
        // If we need more than one page per layer, there is a bug.
        assert!(page < layers.len());

        // Place large items down the left side.
        let mut y = 0;
        loop {
            let layer = &mut layers[large];
            if layer.width() as uint <= WIDTH / 2 || y + layer.height() as uint > HEIGHT {
                break;
            }

            layer.pos = ((page * HEIGHT + y) * WIDTH) as u16;
            for i in range(0, layer.height() as uint) {
                skyline[y + i] = WIDTH as u8 - layer.width();
            }
            y += layer.height() as uint;

            if large == small {
                break 'a;
            }
            large -= 1;
        }
        for i in range(y, HEIGHT) {
            skyline[i] = WIDTH as u8;
        }

        // Place small items down the right side.
        let mut y = 0;
        loop {
            let layer = &mut layers[small];

            while y < HEIGHT && skyline[y] < layer.width() {
                y += 1;
            }

            if y + layer.height() as uint > HEIGHT {
                break;
            }

            let x = 16 - skyline[y];
            layer.pos = ((page * HEIGHT + y) * WIDTH + x as uint) as u16;
            for i in range(0, layer.height() as uint) {
                skyline[y + i] = 16 - (x + layer.width());
            }

            if large == small {
                break 'a;
            }
            small += 1;
        }

        page += 1;
    }

    (page + 1) as i32
}

pub trait Compare<T> {
    fn is_less(&self, a: &T, b: &T) -> bool;
}

struct LayerWH;
impl Compare<Layer> for LayerWH {
    fn is_less(&self, a: &Layer, b: &Layer) -> bool {
        if a.width() != b.width() {
            a.width() < b.width()
        } else {
            // For large items, we don't care how they're ordered by height.  For small items, we
            // want to pack taller items to the left of shorter ones, so sort in descending order
            // by height.
            a.height() > b.height()
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
