#![crate_name = "physics"]
#![no_std]

#![feature(no_std)]
#![feature(core)]
#![feature(static_assert)]

#[macro_use] extern crate core;
#[cfg(asmjs)] #[macro_use] extern crate asmrt;
#[cfg(not(asmjs))] #[macro_use] extern crate std;
#[cfg(not(asmjs))] #[macro_use] extern crate log;

use core::prelude::*;
use core::cmp;
use core::num::SignedInt;

use v3::{Vn, V3, Axis, Region, scalar};


pub mod v3;


// Some macros in `core` rely on names within `::std`.
#[cfg(asmjs)]
mod std {
    pub use core::cmp;
    pub use core::clone;
    pub use core::fmt;
    pub use core::marker;
}


pub const TILE_SIZE: i32 = 32;
pub const TILE_BITS: usize = 5;
pub const TILE_MASK: i32 = TILE_SIZE - 1;
#[allow(dead_code)] #[static_assert]
static TILE_SIZE_BITS: bool = TILE_SIZE == 1 << TILE_BITS as usize;

pub const CHUNK_SIZE: i32 = 16;
pub const CHUNK_BITS: usize = 4;
pub const CHUNK_MASK: i32 = CHUNK_SIZE - 1;
#[allow(dead_code)] #[static_assert]
static CHUNK_SIZE_BITS: bool = CHUNK_SIZE == 1 << CHUNK_BITS as usize;


#[derive(Copy, Eq, PartialEq, Debug, Clone)]
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
    pub fn from_primitive(i: usize) -> Option<Shape> {
        use self::Shape::*;
        let s = match i {
            0 => Empty,
            1 => Floor,
            2 => Solid,
            // TODO: add ramp variants once they are actually supported
            _ => return None,
        };
        Some(s)
    }

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


pub fn collide<S: ShapeSource>(chunk: &S, pos: V3, size: V3, velocity: V3) -> (V3, i32) {
    if velocity == scalar(0) {
        return (pos, core::i32::MAX);
    }

    let end_pos = walk_path(chunk, pos, size, velocity, GroundStep { size: size });

    // Find the actual velocity after adjustment
    let offset = end_pos - pos;
    let abs = offset.abs();
    let max = cmp::max(cmp::max(abs.x, abs.y), abs.z);
    let t =
        if max == 0 {
            0
        } else if max == abs.x {
            offset.x * 1000 / velocity.x
        } else if max == abs.y {
            offset.y * 1000 / velocity.y
        } else {
            offset.z * 1000 / velocity.z
        };

    (end_pos, t)
}


trait StepCallback {
    fn check<S: ShapeSource>(&self, chunk: &S, pos: V3, dir: V3) -> Collision;
}

const AXIS_X: u8 = 1;
const AXIS_Y: u8 = 2;
const AXIS_Z: u8 = 4;

#[derive(PartialEq, Eq, Debug)]
pub enum Collision {
    None,
    Blocked(u8),
    Ramp,
}

fn walk_path<S, CB>(chunk: &S, start_pos: V3, _size: V3, velocity: V3,
                    cb: CB) -> V3
        where S: ShapeSource,
              CB: StepCallback {
    let dir = velocity.signum();
    let mut pos = start_pos;

    let mut last_adj_dir = dir;

    for i in 0..500 {
        // Try up to 4 times to find a direction we can move in.
        let mut adj_dir = dir;
        for _ in 0..4 {
            let collision = cb.check(chunk, pos + adj_dir, adj_dir);
            if collision == Collision::None {
                break;
            } else {
                adj_dir = adjust_direction(adj_dir, collision);
                if adj_dir == scalar(0) {
                    break;
                }
            }
        }

        // Stop if the adjustment changes, sending us in a new direction.  Otherwise, stop if
        // progress is completely blocked.
        if (adj_dir != last_adj_dir && i != 0) || adj_dir == scalar(0) {
            break;
        }

        last_adj_dir = adj_dir;
        pos = pos + adj_dir;
    }

    pos
}

fn adjust_direction(dir: V3, collision: Collision) -> V3 {
    use self::Collision::*;
    match collision {
        None => dir,
        Blocked(axes) => {
            let mut dir = dir;
            if axes & AXIS_X != 0 {
                dir.x = 0;
            }
            if axes & AXIS_Y != 0 {
                dir.y = 0;
            }
            if axes & AXIS_Z != 0 {
                dir.z = 0;
            }
            dir
        },
        Ramp => dir.with(Axis::Z, 1),
    }
}


struct GroundStep {
    size: V3,
}

impl StepCallback for GroundStep {
    fn check<S: ShapeSource>(&self, chunk: &S, pos: V3, dir: V3) -> Collision {
        let bounds = Region::new(pos, pos + self.size);
        // `bounds` converted to block coordinates.
        let bounds_tiles = bounds.div_round(TILE_SIZE);

        let corner = pos + dir.is_positive() * (self.size - scalar(1));

        // Divide the space into sections, like this but in 3 dimensions:
        //
        //    +-+-----+
        //    |3| 2   |
        //    +-+-----+
        //    | |     |
        //    |1| 0   |
        //    | |     |
        //    | |     |
        //    | |     |
        //    +-+-----+
        //
        // The edge and corner sections face in the direction of motion.  This diagrams shows the
        // version for the -x,-y direction.  The section number is a bitfield indicating which axes
        // are 'outside' (the 1-px region in the direction of motion) vs 'inside' (everything else).

        // Bitfield indicating which sections are collided with terrain.
        let mut blocked_sections = 0_u8;

        for tile_pos in bounds_tiles.points() {
            let shape = chunk.get_shape(tile_pos);

            let tile_base_px = tile_pos * scalar(TILE_SIZE);
            let tile_bounds = Region::new(tile_base_px, tile_base_px + scalar(TILE_SIZE));
            let overlap = bounds.intersect(tile_bounds);
            if collide_tile(shape, overlap - tile_base_px) {
                let x_edge = overlap.min.x <= corner.x && corner.x < overlap.max.x;
                let y_edge = overlap.min.y <= corner.y && corner.y < overlap.max.y;
                let z_edge = overlap.min.z <= corner.z && corner.z < overlap.max.z;

                let x_mid = overlap.size().x >= (1 + x_edge as i32);
                let y_mid = overlap.size().y >= (1 + y_edge as i32);
                let z_mid = overlap.size().z >= (1 + z_edge as i32);

                let key = ((x_edge as u8) << 0) |
                          ((y_edge as u8) << 1) |
                          ((z_edge as u8) << 2) |
                          ((x_mid as u8) << 3) |
                          ((y_mid as u8) << 4) |
                          ((z_mid as u8) << 5);

                blocked_sections |= BLOCKED_SECTIONS_TABLE[key as usize];
            }
        }

        // TODO: check for appropriate floor.

        if blocked_sections == 0 {
            Collision::None
        } else if blocked_sections & (1 << 0) != 0 {
            // The correct thing to return here is actually Blocked(7), but that makes it too easy
            // for players to get stuck by placing structures that overlap their current position.
            Collision::None
        } else {
            Collision::Blocked(BLOCKING_TABLE[blocked_sections as usize])
        }
    }
}

fn collide_tile(shape: Shape, _overlap: Region) -> bool {
    shape == Shape::Solid
}


// Generated 2015-05-11 09:04:28 by util/gen_physics_tables.py
const BLOCKED_SECTIONS_TABLE: [u8; 64] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0xc0,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0xa0, 0x00, 0x00, 0x00, 0x00, 0x10, 0x30, 0x50, 0xf0,
    0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x88, 0x00, 0x00, 0x04, 0x0c, 0x00, 0x00, 0x44, 0xcc,
    0x00, 0x02, 0x00, 0x0a, 0x00, 0x22, 0x00, 0xaa, 0x01, 0x03, 0x05, 0x0f, 0x11, 0x33, 0x55, 0xff,
];

// Generated 2015-05-11 09:26:13 by util/gen_physics_tables.py
const BLOCKING_TABLE: [u8; 256] = [
    0x00, 0x07, 0x01, 0x07, 0x02, 0x07, 0x03, 0x07, 0x03, 0x07, 0x01, 0x07, 0x02, 0x07, 0x03, 0x07,
    0x04, 0x07, 0x05, 0x07, 0x06, 0x07, 0x07, 0x07, 0x04, 0x07, 0x05, 0x07, 0x06, 0x07, 0x07, 0x07,
    0x05, 0x07, 0x01, 0x07, 0x02, 0x07, 0x03, 0x07, 0x07, 0x07, 0x01, 0x07, 0x02, 0x07, 0x03, 0x07,
    0x04, 0x07, 0x05, 0x07, 0x06, 0x07, 0x07, 0x07, 0x04, 0x07, 0x05, 0x07, 0x06, 0x07, 0x07, 0x07,
    0x06, 0x07, 0x01, 0x07, 0x02, 0x07, 0x03, 0x07, 0x07, 0x07, 0x01, 0x07, 0x02, 0x07, 0x03, 0x07,
    0x04, 0x07, 0x05, 0x07, 0x06, 0x07, 0x07, 0x07, 0x04, 0x07, 0x05, 0x07, 0x06, 0x07, 0x07, 0x07,
    0x07, 0x07, 0x01, 0x07, 0x02, 0x07, 0x03, 0x07, 0x07, 0x07, 0x01, 0x07, 0x02, 0x07, 0x03, 0x07,
    0x04, 0x07, 0x05, 0x07, 0x06, 0x07, 0x07, 0x07, 0x04, 0x07, 0x05, 0x07, 0x06, 0x07, 0x07, 0x07,
    0x07, 0x07, 0x01, 0x07, 0x02, 0x07, 0x03, 0x07, 0x03, 0x07, 0x01, 0x07, 0x02, 0x07, 0x03, 0x07,
    0x04, 0x07, 0x05, 0x07, 0x06, 0x07, 0x07, 0x07, 0x04, 0x07, 0x05, 0x07, 0x06, 0x07, 0x07, 0x07,
    0x05, 0x07, 0x01, 0x07, 0x02, 0x07, 0x03, 0x07, 0x07, 0x07, 0x01, 0x07, 0x02, 0x07, 0x03, 0x07,
    0x04, 0x07, 0x05, 0x07, 0x06, 0x07, 0x07, 0x07, 0x04, 0x07, 0x05, 0x07, 0x06, 0x07, 0x07, 0x07,
    0x06, 0x07, 0x01, 0x07, 0x02, 0x07, 0x03, 0x07, 0x07, 0x07, 0x01, 0x07, 0x02, 0x07, 0x03, 0x07,
    0x04, 0x07, 0x05, 0x07, 0x06, 0x07, 0x07, 0x07, 0x04, 0x07, 0x05, 0x07, 0x06, 0x07, 0x07, 0x07,
    0x07, 0x07, 0x01, 0x07, 0x02, 0x07, 0x03, 0x07, 0x07, 0x07, 0x01, 0x07, 0x02, 0x07, 0x03, 0x07,
    0x04, 0x07, 0x05, 0x07, 0x06, 0x07, 0x07, 0x07, 0x04, 0x07, 0x05, 0x07, 0x06, 0x07, 0x07, 0x07,
];






/*  old version

trait StepCallback {
    fn check<S: ShapeSource>(&self, chunk: &S, pos: V3, dir_axis: DirAxis) -> bool;

    fn check_post<S: ShapeSource>(&self, _chunk: &S, _pos: V3) -> bool {
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
    //assert!(new.min.x >= 0 && new.min.y >= 0 && new.min.z >= 0);

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

*/




