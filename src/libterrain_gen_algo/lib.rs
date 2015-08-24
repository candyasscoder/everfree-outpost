#![crate_name = "terrain_gen_algo"]
#![allow(dead_code)]
#![feature(
    iter_cmp,
    zero_one,
)]

#[macro_use] extern crate bitflags;
#[macro_use] extern crate log;
extern crate rand;

extern crate server_types as libserver_types;

use libserver_types::*;


pub mod blob;
pub mod cellular;
pub mod disk_sampler;
pub mod dsc;
pub mod pattern;
pub mod triangulate;


pub fn line_points<F: FnMut(V2)>(start: V2, end: V2, mut f: F) {
    // Bresenham line drawing, with some V2 tricks to handle lines running in any direction.
    //
    // A "big step" moves one unit on both the X and Y axes.  A "small step" moves only along the
    // axis that has the greater separation between `start` and `end`.  (If the line is strictly
    // horizontal or vertical, then a big step moves along the one parallel axis and a small step
    // doesn't move at all.)
    let diff = end - start;
    let total = diff.abs().max();
    let big = diff.abs().min();

    let big_dir = diff.signum();
    let small_dir = (diff - big_dir * scalar(big)).signum();

    // `big / total` steps should be big steps, and the rest should be small.  We do this by adding
    // `big` to the accumulator for each stap, and doing a big step each time the accumulator
    // exceeds `total`.  The accumulator starts at `total / 2` so that the two ends of the line are
    // roughly symmetric.
    let mut acc = total / 2;
    let mut pos = start;
    f(pos);
    for _ in 0 .. total {
        acc += big;
        if acc > total {
            acc -= total;
            pos = pos + big_dir;
        } else {
            pos = pos + small_dir;
        }
        f(pos);
    }
}
