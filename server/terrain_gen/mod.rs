use std::cell::RefCell;
use rand::{XorShiftRng, SeedableRng};

use types::*;
use util::StringResult;

use data::Data;
use script::ScriptEngine;

pub use self::disk_sampler::IsoDiskSampler;

mod disk_sampler;


pub struct TerrainGen<'d> {
    data: &'d Data,
    world_seed: u32,
}

impl<'d> TerrainGen<'d> {
    pub fn new(data: &'d Data) -> TerrainGen<'d> {
        TerrainGen {
            data: data,
            world_seed: 0x12345,
        }
    }

    pub fn data(&self) -> &'d Data {
        self.data
    }

    pub fn chunk_rng(&self, cpos: V2, seed: u32) -> XorShiftRng {
        SeedableRng::from_seed([self.world_seed ^ 0xfaa3e2a2,
                                cpos.x as u32,
                                cpos.y as u32,
                                seed])
    }

    pub fn rng(&self, seed: u32) -> XorShiftRng {
        SeedableRng::from_seed([self.world_seed ^ 0x3ba6d154,
                                0x34c9c7b1,
                                0xf8499a88,
                                seed])
    }
}

pub trait Fragment<'d> {
    fn open<F, R>(&mut self, f: F) -> R
        where F: FnOnce(&mut TerrainGen<'d>, &mut ScriptEngine) -> R;

    fn generate(&mut self, cpos: V2) -> StringResult<GenChunk> {
        self.open(|tg, script| {
            let rng = tg.chunk_rng(cpos, 0);
            script.cb_generate_chunk(tg, cpos, rng)
        })
    }
}


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
}

pub struct GenStructure {
    pub pos: V3,
    pub template: TemplateId,
}

impl GenStructure {
    pub fn new(pos: V3, template: TemplateId) -> GenStructure {
        GenStructure {
            pos: pos,
            template: template,
        }
    }
}


pub trait PointSource {
    fn generate_points(&self, bounds: Region2) -> Vec<V2>;
}

pub trait Field {
    fn get_value(&self, pos: V2) -> i32;
}

impl Field for Box<Field> {
    fn get_value(&self, pos: V2) -> i32 {
        (**self).get_value(pos)
    }
}


pub struct ConstantField(pub i32);

impl Field for ConstantField {
    fn get_value(&self, _: V2) -> i32 {
        self.0
    }
}
