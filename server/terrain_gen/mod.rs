use std::cell::RefCell;
use rand::{XorShiftRng, SeedableRng};

use types::*;
use util::StringResult;

use data::Data;
use script::ScriptEngine;


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
}

pub trait Fragment<'d> {
    fn open<F, R>(&mut self, f: F) -> R
        where F: FnOnce(&mut TerrainGen<'d>, &mut ScriptEngine) -> R;

    fn generate(&mut self, cpos: V2) -> StringResult<Box<BlockChunk>> {
        self.open(|tg, script| {
            let rng = SeedableRng::from_seed([cpos.x as u32,
                                              cpos.y as u32,
                                              tg.world_seed,
                                              12345]);
            let mut ctx = Context { data: tg.data };
            script.cb_generate_chunk(&mut ctx, cpos, rng)
        })
    }
}


pub struct Context<'d> {
    data: &'d Data,
}

impl<'d> Context<'d> {
    pub fn data(&self) -> &'d Data {
        self.data
    }
}
