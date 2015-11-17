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

use std::cmp::PartialOrd;
use std::num::Zero;
use std::ops::Add;
use rand::Rng;
use rand::distributions::range::SampleRange;

use libserver_types::*;


pub mod blob;
pub mod cellular;
pub mod disk_sampler;
pub mod dsc;
pub mod pattern;
pub mod triangulate;
pub mod union_find;


pub fn line_points<F: FnMut(V2, bool)>(start: V2, end: V2, mut f: F) {
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
    f(pos, true);
    for _ in 0 .. total {
        acc += big;
        if acc > total {
            acc -= total;
            pos = pos + big_dir;
            f(pos - small_dir, false);
        } else {
            pos = pos + small_dir;
        }
        f(pos, true);
    }
}

pub fn reservoir_sample<R, T, I>(rng: &mut R, mut iter: I) -> Option<T>
        where R: Rng,
              I: Iterator<Item=T> {
    let mut choice = match iter.next() {
        Some(x) => x,
        None => return None,
    };
    let mut count = 1;
    for x in iter {
        count += 1;
        let r = rng.gen_range(0, count);
        if r == 0 {
            choice = x;
        }
    }
    Some(choice)
}

pub fn reservoir_sample_weighted<R, T, W, I>(rng: &mut R, iter: I) -> Option<T>
        where R: Rng,
              W: PartialOrd + SampleRange + Copy + Add<Output=W> + Zero,
              I: Iterator<Item=(T, W)> {
    let mut iter = iter.filter(|&(_, w)| w > Zero::zero());
    let (mut choice, mut weight_sum) = match iter.next() {
        Some(x) => x,
        None => return None,
    };
    for (x, w) in iter {
        weight_sum = weight_sum + w;
        let r = rng.gen_range(Zero::zero(), weight_sum);
        if r < w {
            choice = x;
        }
    }
    Some(choice)
}
