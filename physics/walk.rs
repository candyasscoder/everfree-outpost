use core::cmp;

use v3::{V3, V2, Vn, Axis, scalar, Region};

use super::{Shape, ShapeSource};
use super::TILE_SIZE;
use super::StepCallback;

use self::collision::*;


pub struct GroundStep {
    size: V3,
}

impl GroundStep {
    pub fn new(size: V3) -> GroundStep {
        GroundStep {
            size: size,
        }
    }
}

impl StepCallback for GroundStep {
    fn adjust_offset<S: ShapeSource>(&self, chunk: &S, pos: V3, dir: V3) -> V3 {
        let c1 = self.check_blocked(chunk, pos + dir, dir);
        debug!("{:?}: c1 = {:x}", pos, c1.bits());
        if c1.contains(BLOCKED_BY_RAMP) {
            let adj_dir = self.adjust_planar(chunk, pos, dir.with(Axis::Z, 1));
            debug!("{:?}: upward motion adjusted {:?} -> {:?}",
                   pos, dir.with(Axis::Z, 1), adj_dir);
            if adj_dir != scalar(0) {
                return adj_dir;
            }
        }

        let c2 = self.check_floor(chunk, pos + dir, dir);
        debug!("{:?}: c2 = {:x}", pos, c2.bits());
        if c2.intersects(ABOVE_RAMP | ABOVE_FLOOR) {
            let adj_dir = self.adjust_planar(chunk, pos, dir.with(Axis::Z, -1));
            debug!("{:?}: downward motion adjusted {:?} -> {:?}",
                   pos, dir.with(Axis::Z, -1), adj_dir);
            if adj_dir != scalar(0) {
                return adj_dir;
            }
        }

        let adj_dir1 = apply_collision_planar(dir, c1 | c2.when(!c2.contains(ON_RAMP)));
        debug!("{:?}: planar motion adjusted {:?} -> {:?}",
               pos, dir, adj_dir1);
        let adj_dir2 = self.adjust_planar(chunk, pos, adj_dir1);
        debug!("{:?}: result = {:?}", pos, adj_dir2);
        adj_dir2
    }
}


// Need to wrap Collision in a module so that the `allow(dead_code)` will get applied to the actual
// methods.
mod collision {
    #![allow(dead_code)]
    bitflags! {
        flags Collision: u32 {
            // Blocked by a physical obstacle.
            const BLOCKED_X         = 1 <<  0,
            const BLOCKED_Y         = 1 <<  1,
            const BLOCKED_Z         = 1 <<  2,
            const BLOCKED_MID       = 1 <<  3,

            // Blocked because there's no floor to stand on.
            const NO_FLOOR_X        = 1 <<  4,
            const NO_FLOOR_Y        = 1 <<  5,
            const NO_FLOOR_MID      = 1 <<  7,

            // Blocked by a ramp discontinuity.
            const DISCONT_X         = 1 <<  8,
            const DISCONT_Y         = 1 <<  9,
            const DISCONT_MID       = 1 << 11,

            // All blocking obstacles have Ramp shapes.
            const BLOCKED_BY_RAMP   = 1 << 12,
            // Position is at the top of a ramp.
            const ON_RAMP           = 1 << 13,
            // Position is 1 px above a ramp.
            const ABOVE_RAMP        = 1 << 14,
            // Position is 1 px above a floor.
            const ABOVE_FLOOR       = 1 << 15,

            const BLOCKED_AXIS      = BLOCKED_X.bits
                                    | BLOCKED_Y.bits
                                    | BLOCKED_Z.bits,
            const BLOCKED           = BLOCKED_AXIS.bits
                                    | BLOCKED_MID.bits,

            const NO_FLOOR_AXIS     = NO_FLOOR_X.bits
                                    | NO_FLOOR_Y.bits,
            const NO_FLOOR          = NO_FLOOR_AXIS.bits
                                    | NO_FLOOR_MID.bits,

            const DISCONT_AXIS      = DISCONT_X.bits
                                    | DISCONT_Y.bits,
            const DISCONT           = DISCONT_AXIS.bits
                                    | DISCONT_MID.bits,

            const COLLIDED          = BLOCKED.bits
                                    | NO_FLOOR.bits
                                    | DISCONT.bits,
        }
    }
}

impl Collision {
    fn when(self, cond: bool) -> Collision {
        if cond {
            self
        } else {
            Collision::empty()
        }
    }
}


impl GroundStep {
    // NB: The check_* methods take the *target* position (where the player is currently trying to
    // move).  All other methods take the *current* position (where the player is right now).
    fn check_blocked<S: ShapeSource>(&self, chunk: &S, pos: V3, dir: V3) -> Collision {
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

        let index = adjust_blocked(blocked_sections, axis_mask) as usize;
        Collision::from_bits(BLOCKED_X.bits() * BLOCKING_TABLE[index] as u32).unwrap()
            | BLOCKED_BY_RAMP.when(hit_tiles > 0 && hit_ramps == hit_tiles)
    }

    fn check_floor<S: ShapeSource>(&self, chunk: &S, pos: V3, dir: V3) -> Collision {
        let axis_mask = (((dir.x != 0) as u8) << 0) |
                        (((dir.y != 0) as u8) << 1) |
                        (((dir.z != 0) as u8) << 2);
        let bounds = Region::new(pos, pos + self.size);
        let bounds_tiles = bounds.div_round(TILE_SIZE);
        let corner = pos + dir.is_positive() * (self.size - scalar(1));

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

        let on_floor = pos.z == bounds_tiles.min.z * TILE_SIZE;
        let above_floor = pos.z == bounds_tiles.min.z * TILE_SIZE + 1;
        let mut missing_floor = 0_u8;
        // -2 because -1 could theoretically trigger the `ramp_top == pos.z - 1` case when
        // `pos.z == 0`.
        let mut ramp_top = -2; 

        for tile_pos in bounds_tiles.reduce().points() {
            let shape = chunk.get_shape(tile_pos.extend(bounds_tiles.min.z));

            // Floor only counts if the player is directly on the surface of the floor.
            if shape == Shape::Floor && on_floor {
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
                let new_top = max_altitude(shape, overlap - tile_base_px) +
                        bounds_tiles.min.z * TILE_SIZE;
                ramp_top = cmp::max(new_top, ramp_top);
            } else if shape == Shape::Empty && bounds_tiles.min.z > 0 {
                let shape_below = chunk.get_shape(tile_pos.extend(bounds_tiles.min.z - 1));
                if shape_below.is_ramp() {
                    let new_top = max_altitude(shape_below, overlap - tile_base_px) +
                            (bounds_tiles.min.z - 1) * TILE_SIZE;
                    ramp_top = cmp::max(new_top, ramp_top);
                }
            }
        }

        let index = adjust_blocked(missing_floor, axis_mask) as usize;
        Collision::from_bits(NO_FLOOR_X.bits() * (BLOCKING_TABLE[index] & !AXIS_Z) as u32).unwrap()
            | ABOVE_FLOOR.when(above_floor)
            | ON_RAMP.when(pos.z == ramp_top)
            | ABOVE_RAMP.when(pos.z == ramp_top + 1)
    }

    fn check_ramp<S: ShapeSource>(&self, chunk: &S, pos: V3, dir: V3) -> Collision {
        let axis_mask = (((dir.x != 0) as u8) << 0) |
                        (((dir.y != 0) as u8) << 1) |
                        (((dir.z != 0) as u8) << 2);
        let bounds = Region::new(pos, pos + self.size);
        let bounds_tiles = bounds.div_round(TILE_SIZE);
        let corner = pos + dir.is_positive() * (self.size - scalar(1));

        let blocked = check_ramp_continuity(chunk,
                                            bounds.reduce(),
                                            bounds_tiles.min.z,
                                            corner.reduce());

        let index = adjust_blocked(blocked, axis_mask) as usize;
        Collision::from_bits(DISCONT_X.bits() * (BLOCKING_TABLE[index] & !AXIS_Z) as u32).unwrap()
    }

    fn adjust_planar<S: ShapeSource>(&self, chunk: &S, pos: V3, dir: V3) -> V3 {
        let mut dir = dir;
        for _ in 0..4 {
            // (0, 0, 1) is just as bad as (0, 0, 0)
            if dir.reduce() == scalar(0) {
                break;
            }

            let c1 = self.check_blocked(chunk, pos + dir, dir);
            if c1.intersects(COLLIDED) {
                dir = apply_collision_planar(dir, c1);
                continue;
            }

            let c2 = self.check_floor(chunk, pos + dir, dir);
            if c2.intersects(COLLIDED) && !c2.contains(ON_RAMP) {
                dir = apply_collision_planar(dir, c2);
                continue;
            }

            let c3 = self.check_ramp(chunk, pos + dir, dir);
            if c3.intersects(COLLIDED) {
                dir = apply_collision_planar(dir, c3);
                continue;
            }

            return dir;
        }

        scalar(0)
    }
}

fn apply_collision_planar(dir: V3, c: Collision) -> V3 {
    debug!("apply {:x} to {:?}", c.bits(), dir);

    let mut dir = dir;
    if c.intersects(BLOCKED_X | NO_FLOOR_X | DISCONT_X) {
        dir.x = 0;
    }
    if c.intersects(BLOCKED_Y | NO_FLOOR_Y | DISCONT_Y) {
        dir.y = 0;
    }

    if c.contains(BLOCKED_Z) {
        scalar(0)
    } else {
        dir
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
    use super::Shape::*;
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
    use super::Shape::*;
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
fn check_ramp_continuity<S: ShapeSource>(chunk: &S,
                                         region: Region<V2>,
                                         top_z: i32,
                                         corner: V2) -> u8 {
    let region_tiles = region.div_round(TILE_SIZE);
    let Region { min, max } = region_tiles;

    let mut next_z_x = -1;
    let mut next_z_y = -1;

    //   |   |   |   |
    //   |   |   |   |
    // --+---+---+---+--
    //   |   |   |   |
    //   | *-*---*-* |
    //   | | |   | | |
    // --+-*-*---*-*-+--
    //   | | |   | | |
    //   | | |   | | |
    //   | | |   | | |
    // --+-*-*---*-*-+--
    //   | | |   | | |
    //   | *-*---*-* |
    //   |   |   |   |
    // --+---+---+---+--
    //   |   |   |   |
    //   |   |   |   |
    //
    // The input `region` lies somewhere over the tile grid.  We want to know if the ramp height is
    // continuous across every interior line segment within the region.  That is, for each segment
    // between *s (except those that make up the border of the region), for each of the two
    // endpoints, the altitude should be the same when measured in the tile to each side of the
    // line (for a horizontal line, the tile above and the tile below).

    let mut blocked_sections = 0_u8;

    for p in region_tiles.points() {
        let V2 { x, y } = p;
        // Get z-level to inspect for the current tile.
        let (shape_here, z_here) = chunk.get_shape_below(V3::new(x, y, top_z));
        if x > min.x && next_z_x != z_here {
            // Discontinuity along the left edge of the current tile.
            let line = Region::new(p, p + V2::new(0, 1)) * scalar(TILE_SIZE);
            blocked_sections |= blocked_by_line(line.intersect(region), corner);
        } else if x == min.x && y > min.y && next_z_y != z_here {
            let line = Region::new(p, p + V2::new(1, 0)) * scalar(TILE_SIZE);
            blocked_sections |= blocked_by_line(line.intersect(region), corner);
        }

        // Coordinates within the tile of the intersection of the tile region and the footprint
        // region.  That means these range from [0..32], with numbers other than 0 and 32
        // appearing only at the edges of the region.
        let x0 = if x > min.x { 0 } else { region.min.x - min.x * TILE_SIZE };
        let y0 = if y > min.y { 0 } else { region.min.y - min.y * TILE_SIZE };
        let x1 = if x < max.x - 1 { TILE_SIZE } else { region.max.x - (max.x - 1) * TILE_SIZE};
        let y1 = if y < max.y - 1 { TILE_SIZE } else { region.max.y - (max.y - 1) * TILE_SIZE};
        let alt_here_11 = altitude_at_pixel(shape_here, x1, y1);
        let look_up = alt_here_11 == TILE_SIZE && z_here < top_z;

        // Check the line between this tile and the one to the east, but only the parts that
        // lie within `region`.  (That is, check the eastern border of this tile.)
        if x < max.x - 1 {
            let (shape_right, z_right) = adjacent_shape(chunk, x + 1, y, z_here, look_up);
            let alt_here_10 = altitude_at_pixel(shape_here, TILE_SIZE, y0);
            let alt_right_00 = altitude_at_pixel(shape_right, 0, y0);
            let alt_right_01 = altitude_at_pixel(shape_right, 0, y1);
            if z_here * TILE_SIZE + alt_here_10 != z_right * TILE_SIZE + alt_right_00 ||
               z_here * TILE_SIZE + alt_here_11 != z_right * TILE_SIZE + alt_right_01 {
                // Discontinuity along the eastern edge of this tile.
                let line = Region::new(p + V2::new(1, 0), p + V2::new(1, 1)) * scalar(TILE_SIZE);
                blocked_sections |= blocked_by_line(line.intersect(region), corner);
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
                // Discontinuity along the southern edge of this tile.
                let line = Region::new(p + V2::new(0, 1), p + V2::new(1, 1)) * scalar(TILE_SIZE);
                blocked_sections |= blocked_by_line(line.intersect(region), corner);
            }
            if x == min.x {
                next_z_y = z_down;
            }
        }
    }

    return blocked_sections;

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

fn blocked_by_line(line: Region<V2>, corner: V2) -> u8 {
    let (x_edge, y_edge, x_mid, y_mid) =
        if line.min.y == line.max.y {
            // Horizontal line
            let x_edge = line.min.x <= corner.x && corner.x < line.max.x;
            let y_edge = corner.y == line.min.y || corner.y + 1 == line.min.y;
            let x_mid = (line.max.x - line.min.x) >= (1 + x_edge as i32);
            (x_edge,
             y_edge,
             x_mid,
             !y_edge)
        } else {
            // Vertical line
            let x_edge = corner.x == line.min.x || corner.x + 1 == line.min.x;
            let y_edge = line.min.y <= corner.y && corner.y < line.max.y;
            let y_mid = (line.max.y - line.min.y) >= (1 + y_edge as i32);
            (x_edge,
             y_edge,
             !x_edge,
             y_mid)
        };

    let z_edge = true;
    let z_mid = true;

    let key = ((x_edge as u8) << 0) |
              ((y_edge as u8) << 1) |
              ((z_edge as u8) << 2) |
              ((x_mid as u8) << 3) |
              ((y_mid as u8) << 4) |
              ((z_mid as u8) << 5);

    let result = BLOCKED_SECTIONS_TABLE[key as usize];
    debug!("line {:?}, corner {:?} -> {}, {}, {}, {} -> blocked {:x}",
           line, corner, x_edge as u8, y_edge as u8, x_mid as u8, y_mid as u8, result);
    result
}

fn altitude_at_pixel(shape: Shape, x: i32, y: i32) -> i32 {
    use super::Shape::*;
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

const AXIS_X: u8 = 1 << 0;
const AXIS_Y: u8 = 1 << 1;
const AXIS_Z: u8 = 1 << 2;
