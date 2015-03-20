use std::hash::{Hash, Hasher, SipHasher};
use std::num::Int;
use rand::{Rng, XorShiftRng, SeedableRng};

use types::*;
use util::StringResult;

use data::Data;
use script::ScriptEngine;

pub use self::disk_sampler::IsoDiskSampler;
pub use self::diamond_square::DiamondSquare;
pub use self::fields::{ConstantField, RandomField, FilterField, BorderField};

mod diamond_square;
mod disk_sampler;
mod fields;


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

    fn get_region(&self, bounds: Region2, buf: &mut [i32]) {
        for p in bounds.points() {
            let idx = bounds.index(p);
            buf[idx] = self.get_value(p);
        }
    }
}

impl Field for Box<Field> {
    fn get_value(&self, pos: V2) -> i32 {
        (**self).get_value(pos)
    }

    fn get_region(&self, bounds: Region2, buf: &mut [i32]) {
        (**self).get_region(bounds, buf)
    }
}


struct PointRng {
    seed: u64,
    pos: V2,
    extra: u32,
    counter: u32,
}

impl PointRng {
    pub fn new(seed: u64, pos: V2, extra: u32) -> PointRng {
        PointRng {
            seed: seed,
            pos: pos,
            extra: extra,
            counter: 0,
        }
    }
}

impl Rng for PointRng {
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn next_u64(&mut self) -> u64 {
        let mut hasher = SipHasher::new_with_keys(self.seed, 0x9aa64385cac2f793);
        (self.pos, self.extra, self.counter).hash(&mut hasher);
        self.counter += 1;
        hasher.finish()
    }
}
