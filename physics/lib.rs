#![crate_name = "physics"]
#![no_std]
#![feature(globs, phase)]
#![feature(unboxed_closures)]
#![feature(macro_rules)]

#[phase(plugin, link)] extern crate core;
#[cfg(asmjs)]
#[phase(plugin, link)] extern crate asmrt;
#[cfg(not(asmjs))]
#[phase(plugin, link)] extern crate std;
#[cfg(not(asmjs))]
#[phase(plugin, link)] extern crate log;

use core::prelude::*;
use core::cmp;
use core::num::SignedInt;

use v3::{V3, Axis, DirAxis, Region, scalar};


pub mod v3;


// Some macros in `core` rely on names within `::std`.
#[cfg(asmjs)]
mod std {
    pub use core::cmp;
    pub use core::clone;
    pub use core::fmt;
}


pub const TILE_SIZE: i32 = 32;
pub const TILE_BITS: uint = 5;
pub const TILE_MASK: i32 = TILE_SIZE - 1;
#[allow(dead_code)] #[static_assert]
static TILE_SIZE_BITS: bool = TILE_SIZE == 1 << TILE_BITS as uint;

pub const CHUNK_SIZE: i32 = 16;
pub const CHUNK_BITS: uint = 4;
pub const CHUNK_MASK: i32 = CHUNK_SIZE - 1;
#[allow(dead_code)] #[static_assert]
static CHUNK_SIZE_BITS: bool = CHUNK_SIZE == 1 << CHUNK_BITS as uint;


#[deriving(Eq, PartialEq, Show, Clone)]
#[repr(u8)]
pub enum Shape {
    Empty = 0,
    Floor = 1,
    Solid = 2,
    RampE = 3,
    RampW = 4,
    RampS = 5,
    RampN = 6,
    RampTop = 7,
}

impl Shape {
    pub fn is_ramp(&self) -> bool {
        use self::Shape::*;
        match *self {
            RampE | RampW | RampS | RampN => true,
            _ => false,
        }
    }

    pub fn is_empty(&self) -> bool {
        match *self {
            Shape::Empty |
            Shape::RampTop => true,
            _ => false,
        }
    }
}


pub trait ShapeSource {
    fn get_shape(&self, pos: V3) -> Shape;

    fn get_shape_below(&self, mut pos: V3) -> (Shape, i32) {
        while pos.z >= 0 {
            let s = self.get_shape(pos);
            if !s.is_empty() {
                return (s, pos.z);
            }
            pos.z -= 1;
        }
        (Shape::Empty, 0)
    }
}


trait StepCallback {
    fn check<S: ShapeSource>(&self, chunk: &S, pos: V3, dir_axis: DirAxis) -> bool;

    fn check_post<S: ShapeSource>(&self, chunk: &S, pos: V3) -> bool {
        true
    }
}


struct CheckRegion {
    size: V3,
}

impl CheckRegion {
    fn new(size: V3) -> CheckRegion {
        CheckRegion { size: size }
    }
}

impl StepCallback for CheckRegion {
    #[inline(always)]
    fn check<S: ShapeSource>(&self, chunk: &S, pos: V3, dir_axis: DirAxis) -> bool {
        let CheckRegion { size } = *self;
        let (axis, neg) = dir_axis;

        let edge = pos.get(axis) + size.get_if_pos((axis, neg));

        let min = pos.with(axis, if neg { edge - 1 } else { edge });
        let max = min + size.with(axis, 1);

        if edge % 32 == 0 &&
           !check_region(chunk, Region::new(min, max)) {
            return false;
        }
        true
    }
}


struct CheckRegionSlide {
    base: CheckRegion,
    slide_x: i8,
    slide_y: i8,
    slide_z: i8,
}

impl CheckRegionSlide {
    fn new(size: V3, blocked: V3) -> CheckRegionSlide {
        CheckRegionSlide {
            base: CheckRegion::new(size),
            slide_x: blocked.x as i8,
            slide_y: blocked.y as i8,
            slide_z: blocked.z as i8,
        }
    }
}

impl StepCallback for CheckRegionSlide {
    #[inline(always)]
    fn check<S: ShapeSource>(&self, chunk: &S, pos: V3, dir_axis: DirAxis) -> bool {
        self.base.check(chunk, pos, dir_axis)
    }

    fn check_post<S: ShapeSource>(&self, chunk: &S, pos: V3) -> bool {
        if self.slide_x != 0 {
            if check_side(chunk, pos, self.base.size, (Axis::X, self.slide_x < 0)) {
                return false;
            }
        }

        if self.slide_y != 0 {
            if check_side(chunk, pos, self.base.size, (Axis::Y, self.slide_y < 0)) {
                return false;
            }
        }

        if self.slide_z != 0 {
            if check_side(chunk, pos, self.base.size, (Axis::Z, self.slide_z < 0)) {
                return false;
            }
        }

        true
    }
}


pub fn collide<S: ShapeSource>(chunk: &S, pos: V3, size: V3, velocity: V3) -> (V3, i32) {
    if velocity == scalar(0) {
        return (pos, core::i32::MAX);
    }

    let mut velocity = velocity;

    let mut end_pos = walk_path(chunk, pos, size, velocity, CheckRegion::new(size));
    if end_pos == pos {
        let blocked = blocked_sides(chunk, pos, size, velocity);
        if blocked != velocity.signum() {
            velocity = velocity * blocked.is_zero();
            end_pos = walk_path(chunk, pos, size, velocity,
                                CheckRegionSlide::new(size, blocked));
        }
    }


    let abs = velocity.abs();
    let max = cmp::max(cmp::max(abs.x, abs.y), abs.z);
    let t =
        if max == abs.x {
            (end_pos.x - pos.x) * 1000 / velocity.x
        } else if max == abs.y {
            (end_pos.y - pos.y) * 1000 / velocity.y
        } else {
            (end_pos.z - pos.z) * 1000 / velocity.z
        };

    (end_pos, t)
}


fn blocked_sides<S: ShapeSource>(chunk: &S, pos: V3, size: V3, velocity: V3) -> V3 {
    let neg = velocity.is_negative();
    let blocked_x = !check_side(chunk, pos, size, (Axis::X, neg.x != 0));
    let blocked_y = !check_side(chunk, pos, size, (Axis::Y, neg.y != 0));
    let blocked_z = !check_side(chunk, pos, size, (Axis::Z, neg.z != 0));
    V3::new(if blocked_x { velocity.x.signum() } else { 0 },
            if blocked_y { velocity.y.signum() } else { 0 },
            if blocked_z { velocity.z.signum() } else { 0 })
}

fn check_side<S: ShapeSource>(chunk: &S, pos: V3, size: V3, dir_axis: DirAxis) -> bool {
    let (axis, neg) = dir_axis;
    let edge = pos.get(axis) + size.get_if_pos((axis, neg));
    let min = pos.with(axis, if neg { edge - 1 } else { edge });
    let max = min + size.with(axis, 1);
    let result = check_region(chunk, Region::new(min, max));
    result
}

// `inline(never)` here magically makes `collide` faster.
#[inline(never)]
fn check_region<S: ShapeSource>(chunk: &S, new: Region) -> bool {
    assert!(new.min.x >= 0 && new.min.y >= 0 && new.min.z >= 0);

    // Check that the bottom of the region touches the bottom of the tiles.
    if new.min.z % TILE_SIZE != 0 {
        return false;
    }

    let tile = new.div_round(TILE_SIZE);

    // Check that the bottom layer is all floor.
    for pos in tile.flatten(1).points() {
        if chunk.get_shape(pos) != Shape::Floor {
            return false;
        }
    }

    // Check that the rest of the region is all empty.
    let tile_depth = tile.max.z - tile.min.z;
    let top = tile.flatten(tile_depth - 1) + V3::new(0, 0, 1);
    for pos in top.points() {
        if !chunk.get_shape(pos).is_empty() {
            return false;
        }
    }

    true
}

fn walk_path<S, CB>(chunk: &S, start_pos: V3, size: V3, velocity: V3,
                    cb: CB) -> V3
        where S: ShapeSource,
              CB: StepCallback {
    let mag = velocity.abs();
    let dir = velocity.signum();
    let mut accum = V3::new(0, 0, 0);
    let step_size = cmp::max(cmp::max(mag.x, mag.y), mag.z);

    let mut last_pos = start_pos;

    for _ in range(0u, 500) {
        accum = accum + mag;
        let mut pos = last_pos;

        // I tried using an unboxed closure for most of this instead of a macro, but it caused a
        // 2.5x slowdown due to LLVM not inlining the closure body.  There's no way I could find to
        // give a closure body #[inline(always)], so I used this macro instead.
        macro_rules! maybe_step_axis {
            ($AXIS:ident) => {{
                let axis = Axis::$AXIS;
                if accum.get(axis) >= step_size {
                    // This is simply `accum.$axis -= step_size`.
                    accum = accum.with(axis, accum.get(axis) - step_size);

                    let neg = dir.get(axis) < 0;
                    if !cb.check(chunk, pos, (axis, neg)) {
                        break;
                    }

                    pos = pos + dir.only(axis);
                }}
            }
        }

        maybe_step_axis!(X)
        maybe_step_axis!(Y)
        maybe_step_axis!(Z)

        last_pos = pos;

        if !cb.check_post(chunk, last_pos) {
            break;
        }
    }

    last_pos
}
