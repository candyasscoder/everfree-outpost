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

use v3::{Vn, V3, V2, Axis, Region, scalar};


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
    if velocity == scalar(0) {
        return (pos, core::i32::MAX);
    }

    let end_pos = walk_path(chunk, pos, size, velocity, GroundStep { size: size });

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
    fn check<S: ShapeSource>(&self, chunk: &S, pos: V3, dir: V3) -> Collision;
}

const AXIS_X: u8 = 1;
const AXIS_Y: u8 = 2;
const AXIS_Z: u8 = 4;

#[derive(PartialEq, Eq, Debug)]
pub enum Collision {
    None,
    Blocked(u8),
    RampUp,
    RampDown,
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
        let mut ok = false;
        for _ in 0..4 {
            let collision = cb.check(chunk, pos + adj_dir, adj_dir);
            if collision == Collision::None {
                ok = true;
                break;
            } else {
                let new_adj_dir = adjust_direction(adj_dir, collision);
                if new_adj_dir == adj_dir {
                    // Made no progress.  Looping more is useless.  Leave `ok == false`.
                    break;
                }
                adj_dir = new_adj_dir;
                if adj_dir == scalar(0) {
                    // It's always okay to not move at all.
                    ok = true;
                    break;
                }
            }
        }

        if !ok {
            break;
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
        RampUp => dir.with(Axis::Z, 1),
        RampDown => dir.with(Axis::Z, -1),
    }
}


struct GroundStep {
    size: V3,
}

impl StepCallback for GroundStep {
    fn check<S: ShapeSource>(&self, chunk: &S, pos: V3, dir: V3) -> Collision {
        let axis_mask = (((dir.x != 0) as u8) << 0) |
                        (((dir.y != 0) as u8) << 1) |
                        (((dir.z != 0) as u8) << 2);

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

        // Counts of the number of collided tiles of various types.
        let mut hit_tiles = 0;
        let mut hit_ramps = 0;

        for tile_pos in bounds_tiles.points() {
            let shape = chunk.get_shape(tile_pos);

            let tile_base_px = tile_pos * scalar(TILE_SIZE);
            let tile_bounds = Region::new(tile_base_px, tile_base_px + scalar(TILE_SIZE));
            let overlap = bounds.intersect(tile_bounds);
            if collide_tile(shape, overlap - tile_base_px) {
                hit_tiles += 1;
                if shape.is_ramp() {
                    hit_ramps += 1;
                }

                let key = overlap_key(overlap, corner);
                blocked_sections |= BLOCKED_SECTIONS_TABLE[key as usize];
            }
        }

        // If blocked_sections is 0 (nothing blocked) or 1 (only section #0 blocked), then
        // continue.  If anything else is blocked, return that status.
        if blocked_sections != 0 {
            let index = adjust_blocked(blocked_sections, axis_mask) as usize;
            let axes = BLOCKING_TABLE[index];
            debug!("{:?}: collision: {} / {} ramps, {} / {} axes, {:x} blocked",
                   pos, hit_ramps, hit_tiles, axes, axis_mask, blocked_sections);
            if axes != 0 {
                if hit_ramps == hit_tiles {
                    return Collision::RampUp;
                } else {
                    return Collision::Blocked(axes);
                }
            }
        }

        // There are two permitted statuses with respect to the floor:
        // 1) The player is standing on a completely level floor.
        // 2) The player is standing partway up a ramp, with some part of their outline on the
        //    surface of a ramp.
        //
        // Plus, we need to report two different collision states:
        // 1) Collision::Blocked, if the position is invalid.
        // 2) Collision::RampDown, if the position is invalid but decreasing `pos.z` by one would
        //    make it valid.
        //
        // We collect several measurements of the terrain below the player:
        // 1) `blocked_sections`-like flags indicating which regions have no floor below.
        // 2) Z-coord of the topmost ramp section below the player's outline

        let near_floor = pos.z <= bounds_tiles.min.z * TILE_SIZE + 1;
        let mut missing_floor = 0_u8;
        // -2 because -1 could theoretically trigger the `ramp_top == pos.z - 1` case when
        // `pos.z == 0`.
        let mut ramp_top = -2; 

        for tile_pos in bounds_tiles.reduce().points() {
            let shape = chunk.get_shape(tile_pos.extend(bounds_tiles.min.z));

            // Floor only counts if the player is directly on the surface of the floor.
            if shape == Shape::Floor && near_floor {
                continue;
            }

            let tile_base_px = tile_pos * scalar(TILE_SIZE);
            let tile_bounds = Region::new(tile_base_px, tile_base_px + scalar(TILE_SIZE));
            let overlap = bounds.reduce().intersect(tile_bounds);

            // `extend(0, 2)` to make the `overlap` argument cover both `z_edge` (z == 0) and
            // `z_mid`.  (Basically, there is no actual `z` information for "missing floor"
            // collisions, so just pretend all z-positions are colliding.)
            let key = overlap_key(overlap.extend(0, 2),
                                  corner.with(Axis::Z, 0));
            missing_floor |= BLOCKED_SECTIONS_TABLE[key as usize];

            if shape.is_ramp() {
                debug!("object at {:?} is a ramp", tile_pos.extend(bounds_tiles.min.z));
                let new_top = max_altitude(shape, overlap - tile_base_px) +
                        bounds_tiles.min.z * TILE_SIZE;
                ramp_top = cmp::max(new_top, ramp_top);
            } else if shape == Shape::Empty && bounds_tiles.min.z > 0 {
                debug!("not a ramp, but checking below...");
                let shape_below = chunk.get_shape(tile_pos.extend(bounds_tiles.min.z - 1));
                if shape_below.is_ramp() {
                    debug!("object below at {:?} is a ramp", tile_pos.extend(bounds_tiles.min.z - 1));
                    let new_top = max_altitude(shape_below, overlap - tile_base_px) +
                            (bounds_tiles.min.z - 1) * TILE_SIZE;
                    ramp_top = cmp::max(new_top, ramp_top);
                }
            }
        }

        debug!("{:?}: ramp_top = {}", pos, ramp_top);
        if ramp_top == pos.z || ramp_top == pos.z - 1 {
            let cont = check_ramp_continuity(chunk, bounds);
            let above = ramp_top == pos.z - 1;
            if cont && !above {
                return Collision::None;
            } else if cont && above {
                return Collision::RampDown;
            } else {    // !cont
                // TODO: be more specific based on which side had the discontinuity
                return Collision::Blocked(7);
            }
        }

        if missing_floor != 0 {
            let index = adjust_blocked(missing_floor, axis_mask) as usize;
            let axes = BLOCKING_TABLE[index];
            if axes != 0 {
                return Collision::Blocked(axes);
            }
        }

        // Allow the player to step from z=1 to z=0 if there is floor available.  This lets the
        // player step down from a ramp onto the floor without requiring a check of adjacent tiles.
        // (Pressing "down" at y=31 z=1 will try to move to y=32 z=1, which is 1px above the
        // ground, and not above any ramp-related tiles.  But we notice there is floor at y=32 z=0,
        // so we suggest RampDown.)
        if near_floor && pos.z == bounds_tiles.min.z * TILE_SIZE + 1 {
            return Collision::RampDown;
        }

        Collision::None
    }
}

fn overlap_key(overlap: Region, corner: V3) -> u8 {
    let x_edge = overlap.min.x <= corner.x && corner.x < overlap.max.x;
    let y_edge = overlap.min.y <= corner.y && corner.y < overlap.max.y;
    let z_edge = overlap.min.z <= corner.z && corner.z < overlap.max.z;

    let x_mid = overlap.size().x >= (1 + x_edge as i32);
    let y_mid = overlap.size().y >= (1 + y_edge as i32);
    let z_mid = overlap.size().z >= (1 + z_edge as i32);

    ((x_edge as u8) << 0) |
    ((y_edge as u8) << 1) |
    ((z_edge as u8) << 2) |
    ((x_mid as u8) << 3) |
    ((y_mid as u8) << 4) |
    ((z_mid as u8) << 5)
}

/// Adjust the `blocked` bitmask such that for any "uninteresting" axis (not selected by
/// `axis_mask`), the entire span along that axis is considered `mid`.
fn adjust_blocked(blocked: u8, axis_mask: u8) -> u8 {
    let mut blocked = blocked;
    if axis_mask & AXIS_X == 0 {
        blocked = (blocked | blocked >> 1) & 0x55;
    }
    if axis_mask & AXIS_Y == 0 {
        blocked = (blocked | blocked >> 2) & 0x33;
    }
    if axis_mask & AXIS_Z == 0 {
        blocked = (blocked | blocked >> 4) & 0x0f;
    }
    blocked
}

fn collide_tile(shape: Shape, overlap: Region) -> bool {
    use self::Shape::*;
    match shape {
        Empty => false,
        // Overlap can't represent the possibility of spanning the floor plane in the z axis.
        Floor => false,
        Solid => true,
        r if r.is_ramp() => overlap.min.z < max_altitude(r, overlap.reduce()),
        _ => unreachable!(),
    }
}

fn max_altitude(shape: Shape, overlap: Region<V2>) -> i32 {
    use self::Shape::*;
    match shape {
        // TODO: not sure -1 is a good thing to return here
        Empty => -1,
        Floor => 0,
        Solid => TILE_SIZE,
        RampE => overlap.max.x,
        RampW => TILE_SIZE - overlap.min.x,
        RampS => overlap.max.y,
        RampN => TILE_SIZE - overlap.min.y,
    }
}

// TODO: update to use Region<V2>
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
            let s = chunk.get_shape(V3::new(x, y, z + 1));
            if !s.is_empty() {
                return (s, z + 1);
            }
        }

        let s = chunk.get_shape(V3::new(x, y, z));
        if !s.is_empty() {
            (s, z)
        } else {
            (chunk.get_shape(V3::new(x, y, z - 1)), z - 1)
        }
    }
}

fn altitude_at_pixel(shape: Shape, x: i32, y: i32) -> i32 {
    use self::Shape::*;
    match shape {
        Empty => -1,
        Floor => 0,
        Solid => TILE_SIZE,
        RampE => x,
        RampW => TILE_SIZE - x,
        RampS => y,
        RampN => TILE_SIZE - y,
    }
}


// Generated 2015-05-11 09:04:28 by util/gen_physics_tables.py
const BLOCKED_SECTIONS_TABLE: [u8; 64] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0xc0,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0xa0, 0x00, 0x00, 0x00, 0x00, 0x10, 0x30, 0x50, 0xf0,
    0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x88, 0x00, 0x00, 0x04, 0x0c, 0x00, 0x00, 0x44, 0xcc,
    0x00, 0x02, 0x00, 0x0a, 0x00, 0x22, 0x00, 0xaa, 0x01, 0x03, 0x05, 0x0f, 0x11, 0x33, 0x55, 0xff,
];

// Generated 2015-05-12 20:01:06 by util/gen_physics_tables.py
const BLOCKING_TABLE: [u8; 256] = [
    0x00, 0x00, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03, 0x03, 0x03, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03,
    0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07, 0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07,
    0x05, 0x05, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03, 0x07, 0x07, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03,
    0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07, 0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07,
    0x06, 0x06, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03, 0x07, 0x07, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03,
    0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07, 0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07,
    0x07, 0x07, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03, 0x07, 0x07, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03,
    0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07, 0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07,
    0x07, 0x07, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03, 0x03, 0x03, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03,
    0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07, 0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07,
    0x05, 0x05, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03, 0x07, 0x07, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03,
    0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07, 0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07,
    0x06, 0x06, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03, 0x07, 0x07, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03,
    0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07, 0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07,
    0x07, 0x07, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03, 0x07, 0x07, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03,
    0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07, 0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07,
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




