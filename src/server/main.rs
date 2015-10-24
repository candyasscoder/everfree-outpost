#![crate_name = "backend"]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

#![feature(
    as_slice,   // Option::as_slice
    convert,    // OsStr::to_cstring
    core,
    filling_drop,
    fnbox,
    iter_cmp,
    mpsc_select,
    num_bits_bytes,
    raw,
    scoped,
    step_by,
    trace_macros,
    unboxed_closures,
    unsafe_no_drop_flag,
    vec_push_all,
    vecmap,
    zero_one,
)]

#[macro_use] extern crate bitflags;
extern crate core;
extern crate env_logger;
extern crate libc;
#[macro_use] extern crate log;
extern crate rand;
extern crate rustc_serialize;
extern crate time;

extern crate linked_hash_map;
extern crate lru_cache;
extern crate rusqlite;
extern crate libsqlite3_sys as rusqlite_ffi;

extern crate physics as libphysics;
extern crate terrain_gen as libterrain_gen;
extern crate server_config as libserver_config;
extern crate server_types as libserver_types;
#[macro_use] extern crate server_util as libserver_util;

use std::fs::File;
use std::io::{self, Read};
use rustc_serialize::json;


#[macro_use] mod util;
#[macro_use] mod engine;

mod msg;
mod wire;
mod tasks;
mod timer;
mod types;
mod input;
mod lua;
mod script;
mod world;

mod auth;
mod messages;
mod physics;
mod chunks;
mod terrain_gen;
mod vision;
mod logic;
mod cache;

mod data {
    pub use libserver_config::data::*;
}

mod storage {
    pub use libserver_config::storage::*;
}


fn read_json(mut file: File) -> json::Json {
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    json::Json::from_str(&content).unwrap()
}

fn main() {
    use std::env;
    use std::sync::mpsc::channel;
    use std::thread;

    env_logger::init().unwrap();

    let args = env::args().collect::<Vec<_>>();
    let storage = storage::Storage::new(&args[1]);

    let block_json = read_json(storage.open_block_data());
    let item_json = read_json(storage.open_item_data());
    let recipe_json = read_json(storage.open_recipe_data());
    let template_json = read_json(storage.open_template_data());
    let animation_json = read_json(storage.open_animation_data());
    let loot_table_json = read_json(storage.open_loot_table_data());
    let data = data::Data::from_json(block_json,
                                     item_json,
                                     recipe_json,
                                     template_json,
                                     animation_json,
                                     loot_table_json).unwrap();

    let (req_send, req_recv) = channel();
    let (resp_send, resp_recv) = channel();

    thread::spawn(move || {
        let reader = io::stdin();
        tasks::run_input(reader, req_send).unwrap();
    });

    thread::spawn(move || {
        let writer = io::BufWriter::new(io::stdout());
        tasks::run_output(writer, resp_recv).unwrap();
    });

    let mut engine = engine::Engine::new(&data, &storage, req_recv, resp_send);
    engine.run();
}
