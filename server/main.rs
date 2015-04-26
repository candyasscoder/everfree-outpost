#![crate_name = "backend"]
#![feature(unboxed_closures)]
#![feature(unsafe_destructor)]
#![feature(unsafe_no_drop_flag)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

#![feature(old_path)]
#![feature(old_io)]
#![feature(io)]
#![feature(libc)]
#![feature(path)]
#![feature(core)]
#![feature(collections)]
#![feature(hash)]
#![feature(std_misc)]
#![feature(trace_macros)]

#[macro_use] extern crate bitflags;
extern crate core;
extern crate env_logger;
extern crate libc;
#[macro_use] extern crate log;
extern crate rand;
extern crate "rustc-serialize" as rustc_serialize;
extern crate time;

extern crate collect;
extern crate rusqlite;
extern crate "libsqlite3-sys" as rusqlite_ffi;

extern crate physics;

use std::old_io::{self, File};
use rustc_serialize::json;


#[macro_use] mod util;
#[macro_use] mod engine;

mod msg;
mod wire;
mod tasks;
mod timer;
mod types;
mod input;
mod data;
mod lua;
mod script;
mod world;
mod storage;

mod auth;
mod messages;
// TODO: rename to 'physics'; import lib as 'physics_lib'
mod physics_;
mod chunks;
mod terrain_gen;
mod vision;
mod logic;
mod cache;


fn read_json(mut file: File) -> json::Json {
    let content = file.read_to_string().unwrap();
    json::Json::from_str(&*content).unwrap()
}

fn main() {
    use std::env;
    use std::sync::mpsc::channel;
    use std::thread;

    env_logger::init();

    let args = env::args().collect::<Vec<_>>();
    let storage = storage::Storage::new(Path::new(&args[1]));

    let block_json = read_json(storage.open_block_data());
    let item_json = read_json(storage.open_item_data());
    let recipe_json = read_json(storage.open_recipe_data());
    let template_json = read_json(storage.open_template_data());
    let data = data::Data::from_json(block_json,
                                     item_json,
                                     recipe_json,
                                     template_json).unwrap();

    let (req_send, req_recv) = channel();
    let (resp_send, resp_recv) = channel();

    thread::spawn(move || {
        let reader = old_io::stdin();
        tasks::run_input(reader, req_send).unwrap();
    });

    thread::spawn(move || {
        let writer = old_io::BufferedWriter::new(old_io::stdout());
        tasks::run_output(writer, resp_recv).unwrap();
    });

    let mut engine = engine::Engine::new(&data, &storage, req_recv, resp_send);
    engine.run();
}
