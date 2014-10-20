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
    output.time = check_region_ramp(region, region + input.velocity) as i32;
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

    let end_pos = walk_path(pos, size, velocity);

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
    trace_rect(new.min, new.max - new.min);
    log_arr(&[new.min.z, max_altitude(bottom)]);
    if new.min.z != max_altitude(bottom) {
        return false;
    }

    // Check that there are no collisions.
    let outer = old.join(&new).div_round(TILE_SIZE);
    for pos in outer.points() {
        // The lowest level was implicitly checked for collisions by the altitude code.
        if pos.z == outer.min.z {
            continue;
        }
        log_v3(pos);
        log(get_shape(pos) as i32);
        match get_shape(pos) {
            Empty | RampTop => {},
            _ => return false,
        }
    }

    // Check for continuity of the ramp.

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

static DEFAULT_STEP: uint = 5;

fn walk_path(pos: V3, size: V3, velocity: V3) -> V3 {
    let velocity_gcd = gcd(gcd(velocity.x.abs(), velocity.y.abs()), velocity.z.abs());
    let rel_velocity = velocity / scalar(velocity_gcd);
    let units = cmp::max(cmp::max(rel_velocity.x.abs(), rel_velocity.y.abs()), rel_velocity.z.abs());

    let mut cur = pos * scalar(units);
    let rel_size = size * scalar(units);
    let mut step_size = DEFAULT_STEP;

    reset_trace();

    for i in range(0u, 20) {
        let step = rel_velocity << step_size;
        let next = cur + step;
        let old = Region::new(cur, cur + rel_size);
        let new = Region::new(next, next + rel_size);

        trace_rect(new.min, new.max - new.min);

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
