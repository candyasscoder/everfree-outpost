use core::prelude::*;
use core::cmp;

use v3::{V3, RegionPoints, scalar};
use super::{Shape, get_shape, get_shape_below};
use super::{TILE_SIZE, CHUNK_SIZE};
use super::{gcd, lcm};
use super::{Empty, Floor};
use super::{trace, trace_rect, reset_trace, log, log_arr, log_v3};


#[export_name = "test"]
pub extern fn test(input: &::asmjs::CollideArgs, output: &mut ::CollideResult) {
    *output = collide(input.pos, input.size, input.velocity);
}

fn collide(pos: V3, size: V3, velocity: V3) -> ::CollideResult {
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

fn check_region(min: V3, max: V3) -> bool {
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
    for pos in RegionPoints::new(bottom_min, bottom_max) {
        if get_shape(pos) != Floor {
            return false;
        }
    }

    // Check that the rest of the region is all empty.
    let top_min = V3::new(tile_min.x, tile_min.y, tile_min.z + 1);
    let top_max = tile_max;
    for pos in RegionPoints::new(top_min, top_max) {
        if get_shape(pos) != Empty {
            return false;
        }
    }

    true
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
        // Use + instead of - for min because the components of `step` we're selecting are already
        // negative.
        let mut min = cur + step.is_negative() * step;
        let mut max = cur + step.is_positive() * step + rel_size;

        trace_rect(min, max - min);

        if check_region(min / scalar(units), max / scalar(units)) {
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
