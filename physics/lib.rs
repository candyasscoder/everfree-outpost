#![crate_name = "physics"]
#![no_std]
#![feature(globs, phase)]
#![feature(overloaded_calls, unboxed_closures)]
#![feature(macro_rules)]

#[phase(plugin, link)] extern crate core;
#[cfg(asmjs)]
#[phase(plugin, link)] extern crate asmrt;
#[cfg(not(asmjs))]
#[phase(plugin, link)] extern crate std;

use core::prelude::*;
use core::cmp;

use v3::{V3, Region, scalar};

macro_rules! try_return {
    ($e:expr) => {
        match $e {
            Some(x) => return x,
            None => {},
        }
    }
}

macro_rules! try_return_some {
    ($e:expr) => {
        match $e {
            Some(x) => return Some(x),
            None => {},
        }
    }
}


// Some macros in `core` rely on names within `::std`.
#[cfg(asmjs)]
mod std {
    pub use core::cmp;
    pub use core::fmt;
}


pub mod v3;


fn gcd(mut a: i32, mut b: i32) -> i32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

#[allow(dead_code)]
fn lcm(a: i32, b: i32) -> i32 {
    a * b / gcd(a, b)
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


#[deriving(Eq, PartialEq)]
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
    fn is_ramp(&self) -> bool {
        match *self {
            RampE | RampW | RampS | RampN => true,
            _ => false,
        }
    }
}


pub trait ShapeSource {
    fn get_shape(&self, pos: V3) -> Shape;

    fn get_shape_below(&self, mut pos: V3) -> (Shape, i32) {
        while pos.z >= 0 {
            match self.get_shape(pos) {
                Empty | RampTop => {},
                s => return (s, pos.z),
            }
            pos.z -= 1;
        }
        (Empty, 0)
    }
}

pub fn collide<S: ShapeSource>(chunk: &S, pos: V3, size: V3, velocity: V3) -> (V3, i32) {
    if velocity == scalar(0) {
        return (pos, core::i32::MAX);
    }
    
    let max_velocity = cmp::max(velocity.x.abs(), velocity.y.abs());

    let mut end_pos = walk_path(chunk, pos, size, velocity, check_region);
    if end_pos == pos {
        end_pos = walk_path(chunk, pos, size, velocity, check_region_ramp);
    }
    if end_pos == pos {
        end_pos = walk_path(chunk, pos, size, velocity.with_z(max_velocity), check_region_ramp);
    }
    if end_pos == pos {
        end_pos = walk_path(chunk, pos, size, velocity.with_z(-max_velocity), check_region_ramp);
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


fn check_region<S: ShapeSource>(chunk: &S, old: Region, new: Region) -> bool {
    let Region { min, max } = old.join(&new);

    if min.x < 0 || min.y < 0 || min.z < 0 {
        return false;
    }

    let tile_min = min / scalar(TILE_SIZE);
    let tile_max = (max + scalar(TILE_SIZE - 1)) / scalar(TILE_SIZE);

    // Check that the bottom of the region touches the bottom of the tiles.
    if min.z % TILE_SIZE != 0 {
        return false;
    }

    // Check that the bottom layer is all floor.
    let bottom_min = tile_min;
    let bottom_max = V3::new(tile_max.x, tile_max.y, tile_min.z + 1);
    for pos in Region::new(bottom_min, bottom_max).points() {
        if chunk.get_shape(pos) != Floor {
            return false;
        }
    }

    // Check that the rest of the region is all empty.
    let top_min = V3::new(tile_min.x, tile_min.y, tile_min.z + 1);
    let top_max = tile_max;
    for pos in Region::new(top_min, top_max).points() {
        match chunk.get_shape(pos) {
            Empty | RampTop => {},
            _ => return false,
        }
    }

    true
}

fn check_region_ramp<S: ShapeSource>(chunk: &S, old: Region, new: Region) -> bool {
    if new.min.x < 0 || new.min.y < 0 || new.min.z < 0 {
        return false;
    }

    // Check that we stand at an appropriate altitude.
    // Look both above and below the bottom plane of `min`.  This handles the case where we stand
    // at the very top of a ramp.
    let bottom = new.flatten(2) - V3::new(0, 0, 1);
    if new.min.z != max_altitude(chunk, bottom) {
        return false;
    }

    // Check that we are actually over (or adjacent to) some amount of ramp.
    let expanded = Region::new(bottom.min - V3::new(1, 1, 0),
                               bottom.max + V3::new(1, 1, 0));
    if !expanded.div_round(TILE_SIZE).points().any(|p| chunk.get_shape(p).is_ramp()) {
        return false;
    }

    // Check that there are no collisions.
    let outer_px = old.join(&new);
    let outer = outer_px.div_round(TILE_SIZE);
    for pos in outer.points() {
        // The lowest level was implicitly checked for collisions by the altitude code.
        if pos.z == outer.min.z {
            continue;
        }
        match chunk.get_shape(pos) {
            Empty | RampTop => {},
            _ => return false,
        }
    }

    // Check for continuity of the ramp.
    let mut footprint_px = outer_px;
    footprint_px.min.z = cmp::max(old.min.z, new.min.z);
    if !check_ramp_continuity(chunk, footprint_px) {
        return false;
    }

    true
}

fn max_altitude<S: ShapeSource>(chunk: &S, region: Region) -> i32 {
    let tile_region = region.div_round(TILE_SIZE);
    let mut max_alt = -1;
    for point in tile_region.points() {
        let shape = chunk.get_shape(point);

        let tile_alt = {
            let tile_volume = Region::new(point, point + scalar(1)) * scalar(TILE_SIZE);
            let subregion = region.intersect(&tile_volume) - point * scalar(TILE_SIZE);
            match shape {
                Empty | RampTop => continue,
                Floor => 0,
                Solid => TILE_SIZE,
                RampE => subregion.max.x,
                RampW => TILE_SIZE - subregion.min.x,
                RampS => subregion.max.y,
                RampN => TILE_SIZE - subregion.min.y,
            }
        };

        let alt = tile_alt + point.z * TILE_SIZE;
        if alt > max_alt {
            max_alt = alt;
        }
    }

    max_alt
}

fn altitude_at_pixel(shape: Shape, x: i32, y: i32) -> i32 {
    match shape {
        Empty | RampTop => -1,
        Floor => 0,
        Solid => TILE_SIZE,
        RampE => x,
        RampW => TILE_SIZE - x,
        RampS => y,
        RampN => TILE_SIZE - y,
    }
}

fn check_ramp_continuity<S: ShapeSource>(chunk: &S, region: Region) -> bool {
    let Region { min, max } = region.div_round(TILE_SIZE);
    let top_z = min.z;

    let mut next_z_x = -1;
    let mut next_z_y = -1;

    for y in range(min.y, max.y) {
        for x in range(min.x, max.x) {
            // Get z-level to inspect for the current tile.
            let (shape_here, z_here) = chunk.get_shape_below(V3::new(x, y, top_z));
            if x > min.x && next_z_x != z_here {
                return false;
            } else if x == min.x && y > min.y && next_z_y != z_here {
                return false;
            }

            // Coordinates within the tile of the intersection of the tile region and the footprint
            // region.  That means these range from [0..32], with numbers other than 0 and 32
            // appearing only at the edges.
            let x0 = if x > min.x { 0 } else { region.min.x - min.x * TILE_SIZE };
            let y0 = if y > min.y { 0 } else { region.min.y - min.y * TILE_SIZE };
            let x1 = if x < max.x - 1 { TILE_SIZE } else { region.max.x - (max.x - 1) * TILE_SIZE};
            let y1 = if y < max.y - 1 { TILE_SIZE } else { region.max.y - (max.y - 1) * TILE_SIZE};
            let alt_here_11 = altitude_at_pixel(shape_here, x1, y1);
            let look_up = alt_here_11 == TILE_SIZE && z_here < top_z;

            // Check the line between this tile and the one to the east, but only the parts that
            // lie within `region`.
            if x < max.x - 1 {
                let (shape_right, z_right) = adjacent_shape(chunk, x + 1, y, z_here, look_up);
                let alt_here_10 = altitude_at_pixel(shape_here, TILE_SIZE, y0);
                let alt_right_00 = altitude_at_pixel(shape_right, 0, y0);
                let alt_right_01 = altitude_at_pixel(shape_right, 0, y1);
                if z_here * TILE_SIZE + alt_here_10 != z_right * TILE_SIZE + alt_right_00 ||
                   z_here * TILE_SIZE + alt_here_11 != z_right * TILE_SIZE + alt_right_01 {
                    return false;
                }
                // Save the z that we expect to see when visiting the tile to the east.  If we see
                // a different z, then there's a problem like this:
                //   _ /
                //    \_
                // The \ ramp is properly connected to the rightmost _, but the / ramp is actually
                // the topmost tile in that column.
                next_z_x = z_right;
            }

            // Check the line between this tile and the one to the south.
            if y < max.y - 1 {
                let (shape_down, z_down) = adjacent_shape(chunk, x, y + 1, z_here, look_up);
                let alt_here_01 = altitude_at_pixel(shape_here, x0, TILE_SIZE);
                let alt_down_00 = altitude_at_pixel(shape_down, x0, 0);
                let alt_down_10 = altitude_at_pixel(shape_down, x1, 0);
                if z_here * TILE_SIZE + alt_here_01 != z_down * TILE_SIZE + alt_down_00 ||
                   z_here * TILE_SIZE + alt_here_11 != z_down * TILE_SIZE + alt_down_10 {
                    return false;
                }
                if x == min.x {
                    next_z_y = z_down;
                }
            }
        }
    }

    return true;

    fn adjacent_shape<S: ShapeSource>(chunk: &S, x: i32, y: i32, z: i32,
                                      look_up: bool) -> (Shape, i32) {
        if look_up {
            match chunk.get_shape(V3::new(x, y, z + 1)) {
                Empty | RampTop => {},
                s => return (s, z + 1),
            }
        }

        match chunk.get_shape(V3::new(x, y, z)) {
            Empty | RampTop => (chunk.get_shape(V3::new(x, y, z - 1)), z - 1),
            s => return (s, z),
        }
    }
}

const DEFAULT_STEP: uint = 5;

fn walk_path<S: ShapeSource>(chunk: &S, pos: V3, size: V3, velocity: V3,
                             check_region: |&S, Region, Region| -> bool) -> V3 {
    let velocity_gcd = gcd(gcd(velocity.x.abs(), velocity.y.abs()), velocity.z.abs());
    let rel_velocity = velocity / scalar(velocity_gcd);
    let units = cmp::max(cmp::max(rel_velocity.x.abs(), rel_velocity.y.abs()), rel_velocity.z.abs());

    let mut cur = pos * scalar(units);
    let rel_size = size * scalar(units);
    let mut step_size = DEFAULT_STEP;

    //reset_trace();

    for _ in range(0u, 20) {
        let step = rel_velocity << step_size;
        let next = cur + step;
        let old = Region::new(cur, cur + rel_size);
        let new = Region::new(next, next + rel_size);

        //trace_rect(new.min, new.max - new.min);

        if check_region(chunk, old, new) {
            cur = cur + step;
            if step_size < DEFAULT_STEP {
                step_size += 1;
            }
        } else {
            if step_size > 0 {
                step_size -= 1;
            } else {
                break;
            }
        }
    }

    cur / scalar(units)
}
