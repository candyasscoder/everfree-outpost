#![crate_name = "terrain_gen"]
#![allow(dead_code)]

#[macro_use] extern crate bitflags;
extern crate linked_hash_map;
#[macro_use] extern crate log;
extern crate rand;
extern crate time;

extern crate physics as libphysics;
extern crate server_config as libserver_config;
extern crate server_types as libserver_types;
extern crate server_util as libserver_util;
extern crate terrain_gen_algo as libterrain_gen_algo;

use std::collections::HashMap;
use rand::XorShiftRng;

use libphysics::CHUNK_SIZE;
use libserver_types::*;

pub use libterrain_gen_algo as algo;

pub mod worker;
mod prop;
mod cache;

mod forest;
mod dungeon;


pub type StdRng = XorShiftRng;


pub struct GenChunk {
    pub blocks: Box<BlockChunk>,
    pub structures: Vec<GenStructure>,
}

impl GenChunk {
    pub fn new() -> GenChunk {
        GenChunk {
            blocks: Box::new(EMPTY_CHUNK),
            structures: Vec::new(),
        }
    }

    pub fn set_block(&mut self, pos: V3, val: BlockId) {
        let bounds = Region::new(scalar(0), scalar(CHUNK_SIZE));
        assert!(bounds.contains(pos));
        self.blocks[bounds.index(pos)] = val;
    }

    pub fn get_block(&self, pos: V3) -> BlockId {
        let bounds = Region::new(scalar(0), scalar(CHUNK_SIZE));
        assert!(bounds.contains(pos));
        self.blocks[bounds.index(pos)]
    }
}

pub struct GenStructure {
    pub pos: V3,
    pub template: TemplateId,
    pub extra: HashMap<String, String>,
}

impl GenStructure {
    pub fn new(pos: V3, template: TemplateId) -> GenStructure {
        GenStructure {
            pos: pos,
            template: template,
            extra: HashMap::new(),
        }
    }
}
