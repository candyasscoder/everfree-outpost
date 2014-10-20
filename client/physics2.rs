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
    //let region = Region::new(input.pos, input.pos + input.size);
    //output.time = check_region_ramp(region, region + input.velocity) as i32;
    output.time = corner_smooth(input.pos) as i32;
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
    // TODO: need to check lines instead of points, in order to handle trying to walk up a ramp
    // while halfway off of it (currently lets you go up 1px, since the bottom corner is
    // technically continuous)
    let inner_footprint = outer_px.expand(&scalar(-1)).flatten(1);
    let footprint_z = cmp::max(old.min.z, new.min.z);
    let mut corner_footprint = inner_footprint.div_round(TILE_SIZE);
    corner_footprint.max.x += 1;
    corner_footprint.max.y += 1;
    let corner_footprint = corner_footprint;
    for point in corner_footprint.points().map(|p| p * scalar(TILE_SIZE)) {
        let clamped = inner_footprint.clamp_point(&point).with_z(footprint_z);
        if !corner_smooth(clamped) {
            return false;
        }
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

fn corner_smooth(point: V3) -> bool {
    fn altitude_modified(point: V3, dir: V3) -> i32 {
        let base = (point - dir) / scalar(TILE_SIZE);
        let offset = point - base * scalar(TILE_SIZE);

        let (shape, z) = get_shape_below(base);
        let alt = altitude_at_pixel(shape, offset.x, offset.y);
        //log_arr(&[999, point.x, point.y, point.z, dir.x, dir.y, dir.z, alt]);
        //log_arr(&[999, base.x, base.y, base.z, offset.x, offset.y, offset.z, z]);
        if alt == -1 {
            -1
        } else {
            z * TILE_SIZE + alt
        }
    }

    let a = altitude_modified(point, V3::new(0, 0, 0));
    let b = altitude_modified(point, V3::new(0, 1, 0));
    let c = altitude_modified(point, V3::new(1, 0, 0));
    let d = altitude_modified(point, V3::new(1, 1, 0));

    //log_arr(&[point.x, point.y, point.z, a, b, c, d]);

    a == b && b == c && c == d
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
