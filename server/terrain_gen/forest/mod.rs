use rand::Rng;

use types::*;


mod summary;
mod super_heightmap;
mod heightmap;
mod caves;
mod trees;
mod provider;

pub use self::provider::Provider;


fn div_rand_round<R: Rng>(r: &mut R, n: u32, d: u32) -> u32 {
    (n + r.gen_range(0, d)) / d
}

fn power_from_dist<R: Rng>(r: &mut R, dist: i32) -> u8 {
    fn ramp<R: Rng>(r: &mut R, x: u32, min_x: u32, max_x: u32, min_y: u32, max_y: u32) -> u32 {
        let x_range = max_x - min_x;
        let y_range = max_y - min_y;
        let dx = x - min_x;
        let dy = div_rand_round(r, dx * y_range, x_range);
        min_y + dy
    }

    let dist = dist as u32;
    if dist < 256 {
        ramp(r, dist, 0, 256, 0, 1) as u8
    } else if dist < 512 {
        ramp(r, dist, 256, 512, 1, 4) as u8
    } else if dist < 1024 {
        ramp(r, dist, 512, 1024, 4, 15) as u8
    } else {
        15
    }
}

pub fn power<R: Rng>(rng: &mut R, cpos: V2) -> u8 {
    power_from_dist(rng, cpos.abs().max())
}

pub fn exp_power<R: Rng>(rng: &mut R, cpos: V2) -> u8 {
    (15 - power(rng, cpos)).leading_zeros() as u8 - (8 - 4)
}
