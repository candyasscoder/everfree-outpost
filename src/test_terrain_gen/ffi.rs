#![crate_name = "terrain_gen_ffi"]
#![crate_type = "staticlib"]
#![feature(
    box_raw,
    cstr_to_str,
    libc,
    scoped,
)]

extern crate env_logger;
extern crate libc;
extern crate rustc_serialize;
extern crate terrain_gen as libterrain_gen;
extern crate server_config as libserver_config;
extern crate server_types as libserver_types;

use std::collections::hash_map;
use std::fs::File;
use std::ffi::CStr;
use std::io::Read;
use std::mem;
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread::{self, JoinGuard};
use libc::{c_char, size_t};
use rustc_serialize::json;

use libserver_config::{Data, Storage};
use libserver_types::*;
use libterrain_gen::{GenChunk, GenStructure};
use libterrain_gen::worker::{self, Command, Response};

#[allow(dead_code)]
pub struct Worker {
    send: Sender<Command>,
    recv: Receiver<Response>,
    data: Box<Data>,
    storage: Box<Storage>,
    guard: JoinGuard<'static, ()>,
}

fn read_json(mut file: File) -> json::Json {
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    json::Json::from_str(&content).unwrap()
}

impl Worker {
    fn new(path: &str) -> Worker {
        let storage = Box::new(Storage::new(&path.to_owned()));

        let block_json = read_json(storage.open_block_data());
        let item_json = read_json(storage.open_item_data());
        let recipe_json = read_json(storage.open_recipe_data());
        let template_json = read_json(storage.open_template_data());
        let animation_json = read_json(storage.open_animation_data());
        let data = Box::new(Data::from_json(block_json,
                                            item_json,
                                            recipe_json,
                                            template_json,
                                            animation_json).unwrap());

        let (send_cmd, recv_cmd) = mpsc::channel();
        let (send_result, recv_result) = mpsc::channel();
        // Make sure the closure only looks at the heap-allocated storage, not the stack-allocated
        // boxes themselves.
        let guard = {
            let storage_ref: &Storage = &*storage;
            let data_ref: &Data = &*data;
            let guard = thread::scoped(move || {
                worker::run(data_ref, storage_ref, recv_cmd, send_result);
            });
            // Cast away the lifetimes so we can move `data` and `storage` into the struct.
            unsafe { mem::transmute(guard) }
        };

        Worker {
            send: send_cmd,
            recv: recv_result,
            data: data,
            storage: storage,
            guard: guard,
        }
    }
}


static mut INITED_LOGGER: bool = false;

fn init_logger() {
    unsafe {
        if !INITED_LOGGER {
            env_logger::init().unwrap();
            INITED_LOGGER = true;
        }
    }
}


#[no_mangle]
pub unsafe extern "C" fn worker_create(path: *const c_char) -> *mut Worker {
    init_logger();
    let c_str = CStr::from_ptr(path);
    let s = c_str.to_str().unwrap();
    let ptr = Box::new(Worker::new(s));
    Box::into_raw(ptr)
}

#[no_mangle]
pub unsafe extern "C" fn worker_destroy(ptr: *mut Worker) {
    drop(Box::from_raw(ptr));
}

#[no_mangle]
pub unsafe extern "C" fn worker_request(ptr: *mut Worker, pid: u64, x: i32, y: i32) {
    let cmd = Command::Generate(Stable::new(pid), V2::new(x, y));
    (*ptr).send.send(cmd).unwrap();
}

#[no_mangle]
pub unsafe extern "C" fn worker_get_response(ptr: *mut Worker,
                                             pid_p: *mut u64,
                                             x_p: *mut i32,
                                             y_p: *mut i32) -> *mut GenChunk {
    let (pid, pos, gc) = (*ptr).recv.recv().unwrap();
    *pid_p = pid.unwrap();
    *x_p = pos.x;
    *y_p = pos.y;
    Box::into_raw(Box::new(gc))
}


#[no_mangle]
pub unsafe extern "C" fn chunk_free(ptr: *mut GenChunk) {
    drop(Box::from_raw(ptr))
}

#[no_mangle]
pub unsafe extern "C" fn chunk_blocks_len(ptr: *const GenChunk) -> size_t {
    (*ptr).blocks.len() as size_t
}

#[no_mangle]
pub unsafe extern "C" fn chunk_get_block(ptr: *const GenChunk, idx: size_t) -> BlockId {
    (*ptr).blocks[idx as usize]
}

#[no_mangle]
pub unsafe extern "C" fn chunk_structures_len(ptr: *const GenChunk) -> size_t {
    (*ptr).structures.len() as size_t
}

#[no_mangle]
pub unsafe extern "C" fn chunk_get_structure(ptr: *const GenChunk,
                                             idx: size_t) -> *const GenStructure {
    &(*ptr).structures[idx as usize]
}


#[no_mangle]
pub unsafe extern "C" fn structure_get_pos(ptr: *const GenStructure,
                                           x_p: *mut i32,
                                           y_p: *mut i32,
                                           z_p: *mut i32) {
    *x_p = (*ptr).pos.x;
    *y_p = (*ptr).pos.y;
    *z_p = (*ptr).pos.z;
}

#[no_mangle]
pub unsafe extern "C" fn structure_get_template(ptr: *const GenStructure) -> TemplateId {
    (*ptr).template
}

#[no_mangle]
pub unsafe extern "C" fn structure_extra_len(ptr: *const GenStructure) -> size_t {
    (*ptr).extra.len() as size_t
}

#[no_mangle]
pub unsafe extern "C" fn structure_extra_iter(ptr: *const GenStructure) -> *mut ExtraIter {
    Box::into_raw(Box::new((*ptr).extra.iter()))
}


pub type ExtraIter = hash_map::Iter<'static, String, String>;

#[no_mangle]
pub unsafe extern "C" fn extra_iter_free(ptr: *mut ExtraIter) {
    drop(Box::from_raw(ptr))
}

#[no_mangle]
pub unsafe extern "C" fn extra_iter_next(ptr: *mut ExtraIter,
                                         key_p: *mut *const c_char,
                                         key_len_p: *mut size_t,
                                         value_p: *mut *const c_char,
                                         value_len_p: *mut size_t) -> bool {
    match (*ptr).next() {
        None => false,
        Some((k, v)) => {
            *key_p = k.as_ptr() as *const c_char;
            *key_len_p = k.len() as size_t;
            *value_p = v.as_ptr() as *const c_char;
            *value_len_p = v.len() as size_t;
            true
        },
    }
}

