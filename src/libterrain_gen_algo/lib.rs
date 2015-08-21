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


pub mod cellular;
pub mod disk_sampler;
pub mod dsc;
pub mod pattern;
