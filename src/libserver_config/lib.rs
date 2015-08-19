#![crate_name = "server_config"]

#[macro_use] extern crate log;
extern crate physics as libphysics;
extern crate server_types as libserver_types;
extern crate rustc_serialize;

pub mod data;
pub mod storage;
