use core::prelude::*;
use core::cmp;

use v3::{V3, Region, scalar};
use super::{Shape, get_shape, get_shape_below};
use super::{TILE_SIZE, CHUNK_SIZE};
use super::{gcd, lcm};
use super::{Empty, Floor, RampTop};
use super::{trace, trace_rect, reset_trace, log, log_arr, log_v3};


#[export_name = "test"]
pub extern fn test(input: &::asmjs::CollideArgs, output: &mut ::CollideResult) {
    let region = Region::new(input.pos, input.pos + input.size);
    //output.time = check_region_ramp(region, region + input.velocity) as i32;
    output.time = check_ramp_continuity(region) as i32;
}

pub fn collide(pos: V3, size: V3, velocity: V3) -> ::CollideResult {
    if velocity == scalar(0) {
        return ::CollideResult {
            pos: pos,
            time: 0,
            dirs: 0,
            reason: ::Wall,
        };
    }
    
    let max_velocity = cmp::max(velocity.x.abs(), velocity.y.abs());

    let mut end_pos = walk_path(pos, size, velocity, |&:old, new| check_region(old, new));
    if end_pos == pos {
        end_pos = walk_path(pos, size, velocity, |&:old, new| check_region_ramp(old, new));
    }
    if end_pos == pos {
        end_pos = walk_path(pos, size, velocity.with_z(max_velocity), |&:old, new| check_region_ramp(old, new));
    }
    if end_pos == pos {
        end_pos = walk_path(pos, size, velocity.with_z(-max_velocity), |&:old, new| check_region_ramp(old, new));
    }
    let r = Region::new(end_pos, end_pos + size);
    check_region_ramp(r, r);

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
    ::CollideResult {
        pos: end_pos,
        time: t,
        dirs: 0,
        reason: ::Wall,
    }
}


struct State;

fn check_region(old: Region, new: Region) -> bool {
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
        if get_shape(pos) != Floor {
            return false;
        }
    }

    // Check that the rest of the region is all empty.
    let top_min = V3::new(tile_min.x, tile_min.y, tile_min.z + 1);
    let top_max = tile_max;
    for pos in Region::new(top_min, top_max).points() {
        match get_shape(pos) {
            Empty | RampTop => {},
            _ => return false,
        }
    }

    true
}

fn check_region_ramp(old: Region, new: Region) -> bool {
    use {Empty, Floor, Solid, RampE, RampS, RampW, RampN, RampTop};

    if new.min.x < 0 || new.min.y < 0 || new.min.z < 0 {
        return false;
    }

    // Check that we stand at an appropriate altitude.
    // Look both above and below the bottom plane of `min`.  This handles the case where we stand
    // at the very top of a ramp.
    let bottom = new.flatten(2) - V3::new(0, 0, 1);
    if new.min.z != max_altitude(bottom) {
        return false;
    }

    // Check that we are actually over (or adjacent to) some amount of ramp.
    let expanded = Region::new(bottom.min - V3::new(1, 1, 0),
                               bottom.max + V3::new(1, 1, 0));
    if !expanded.div_round(TILE_SIZE).points().any(|p| get_shape(p).is_ramp()) {
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
        match get_shape(pos) {
            Empty | RampTop => {},
            _ => return false,
        }
    }

    // Check for continuity of the ramp.
    let mut footprint_px = outer_px;
    footprint_px.min.z = cmp::max(old.min.z, new.min.z);
    if !check_ramp_continuity(footprint_px) {
        return false;
    }

    true
}

fn max_altitude(region: Region) -> i32 {
    let tile_region = region.div_round(TILE_SIZE);
    let inner_region = Region::new(tile_region.min + V3::new(1, 1, 0),
                                   tile_region.max - V3::new(1, 1, 0));
    let mut max_alt = -1;
    for point in tile_region.points() {
        let shape = get_shape(point);

        let tile_alt = {
            use {Empty, Floor, Solid, RampE, RampS, RampW, RampN, RampTop};
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
    use {Empty, Floor, Solid, RampE, RampS, RampW, RampN, RampTop};
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

fn check_ramp_continuity(region: Region) -> bool {
    let Region { min, max } = region.div_round(TILE_SIZE);
    let top_z = min.z;

    let mut next_z_x = -1;
    let mut next_z_y = -1;

    for y in range(min.y, max.y) {
        for x in range(min.x, max.x) {
            // Get z-level to inspect for the current tile.
            let (shape_here, z_here) = get_shape_below(V3::new(x, y, top_z));
            if x > min.x && next_z_x != z_here {
                return false;
            } else if x == min.x && y > min.y && next_z_y != z_here {
                return false;
            }

            let shape_here = get_shape(V3::new(x, y, z_here));
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
                let (shape_right, z_right) = adjacent_shape(x + 1, y, z_here, look_up);
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
                let (shape_down, z_down) = adjacent_shape(x, y + 1, z_here, look_up);
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

    fn adjacent_shape(x: i32, y: i32, z: i32, look_up: bool) -> (Shape, i32) {
        if look_up {
            match get_shape(V3::new(x, y, z + 1)) {
                Empty | RampTop => {},
                s => return (s, z + 1),
            }
        }

        match get_shape(V3::new(x, y, z)) {
            Empty | RampTop => (get_shape(V3::new(x, y, z - 1)), z - 1),
            s => return (s, z),
        }
    }
}

static DEFAULT_STEP: uint = 5;

fn walk_path<F>(pos: V3, size: V3, velocity: V3,
                check_region: F) -> V3
        where F: Fn(Region, Region) -> bool {
    let velocity_gcd = gcd(gcd(velocity.x.abs(), velocity.y.abs()), velocity.z.abs());
    let rel_velocity = velocity / scalar(velocity_gcd);
    let units = cmp::max(cmp::max(rel_velocity.x.abs(), rel_velocity.y.abs()), rel_velocity.z.abs());

    let mut cur = pos * scalar(units);
    let rel_size = size * scalar(units);
    let mut step_size = DEFAULT_STEP;

    //reset_trace();

    for i in range(0u, 20) {
        let step = rel_velocity << step_size;
        let next = cur + step;
        let old = Region::new(cur, cur + rel_size);
        let new = Region::new(next, next + rel_size);

        //trace_rect(new.min, new.max - new.min);

        if check_region(old, new) {
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
