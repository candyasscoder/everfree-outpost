#![crate_name = "physics"]
#![no_std]

#![feature(no_std)]
#![feature(core)]
#![feature(static_assert)]

#[macro_use] extern crate core;
#[macro_use] extern crate bitflags;
#[cfg(asmjs)] #[macro_use] extern crate asmrt;
#[cfg(not(asmjs))] #[macro_use] extern crate std;
#[cfg(not(asmjs))] #[macro_use] extern crate log;

use core::prelude::*;
use core::num::SignedInt;

use v3::{Vn, V3, scalar};


pub mod v3;
mod walk;


// Some macros in `core` rely on names within `::std`.
#[cfg(asmjs)]
mod std {
    pub use core::*;
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
}

impl Shape {
    pub fn from_primitive(i: usize) -> Option<Shape> {
        use self::Shape::*;
        let s = match i {
            0 => Empty,
            1 => Floor,
            2 => Solid,
            6 => RampN,
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
            Shape::Empty => true,
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
    use walk::GroundStep;

    if velocity == scalar(0) {
        return (pos, core::i32::MAX);
    }

    let end_pos = walk_path(chunk, pos, size, velocity, GroundStep::new(size));

    // Find the actual velocity after adjustment
    let velocity_mag = velocity.abs().max();
    let offset_mag = (end_pos - pos).abs().max();
    let t =
        if velocity_mag == 0 {
            0
        } else {
            offset_mag * 1000 / velocity_mag
        };

    (end_pos, t)
}


trait StepCallback {
    fn adjust_offset<S: ShapeSource>(&self, chunk: &S, pos: V3, dir: V3) -> V3;
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
        let adj_dir = cb.adjust_offset(chunk, pos, dir);

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
