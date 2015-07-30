use std::collections::HashMap;
use std::hash::{Hash, Hasher, SipHasher};
use std::mem;
use std::thread::{self, JoinGuard};
use std::sync::mpsc::{self, Sender, Receiver};
use rand::{Rng, XorShiftRng, SeedableRng};

use physics::CHUNK_SIZE;
use types::*;
use util::StrResult;

use data::Data;
use storage::Storage;
use world::Fragment as World_Fragment;
use world::Hooks;
use world::flags;
use world::object::*;

pub use self::disk_sampler::IsoDiskSampler;
pub use self::diamond_square::DiamondSquare;
pub use self::fields::{ConstantField, RandomField, FilterField, BorderField};

use self::summary::Summary;
use self::dsc::DscGrid;

mod diamond_square;
mod disk_sampler;
mod fields;
mod summary;
mod dsc;
mod worker;


pub type TerrainGenEvent = worker::Response;

pub struct TerrainGen<'d> {
    send: Sender<worker::Command>,
    recv: Receiver<worker::Response>,
    guard: JoinGuard<'d, ()>,
}

impl<'d> TerrainGen<'d> {
    pub fn new(data: &'d Data, storage: &'d Storage) -> TerrainGen<'d> {
        let (send_cmd, recv_cmd) = mpsc::channel();
        let (send_result, recv_result) = mpsc::channel();
        let guard = thread::scoped(move || {
            worker::run(data, storage, recv_cmd, send_result);
        });

        TerrainGen {
            send: send_cmd,
            recv: recv_result,
            guard: guard,
        }
    }

    pub fn receiver(&self) -> &Receiver<TerrainGenEvent> {
        &self.recv
    }
}

pub trait Fragment<'d> {
    fn terrain_gen_mut(&mut self) -> &mut TerrainGen<'d>;

    type WF: World_Fragment<'d>;
    fn with_world<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Self::WF) -> R;

    fn generate(&mut self,
                pid: PlaneId,
                cpos: V2) -> StrResult<TerrainChunkId> {
        let stable_pid = self.with_world(|wf| wf.plane_mut(pid).stable_id());
        self.terrain_gen_mut().send.send(worker::Command::Generate(stable_pid, cpos)).unwrap();
        self.with_world(move |wf| { wf.create_terrain_chunk(pid, cpos).map(|tc| tc.id()) })
    }

    fn process(&mut self, evt: TerrainGenEvent) {
        let (stable_pid, cpos, gc) = evt;
        self.with_world(move |wf| {
            let tcid = {
                let pid = unwrap_or!(wf.world().transient_plane_id(stable_pid));
                let mut p = wf.plane_mut(pid);
                let mut tc = unwrap_or!(p.get_terrain_chunk_mut(cpos));
                *tc.blocks_mut() = *gc.blocks;
                tc.flags_mut().remove(flags::TC_GENERATION_PENDING);
                tc.id()
            };
            wf.with_hooks(|h| h.on_terrain_chunk_update(tcid));
        });
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

    pub fn set_block(&mut self, pos: V3, val: BlockId) {
        let bounds = Region::new(scalar(0), scalar(CHUNK_SIZE));
        assert!(bounds.contains(pos));
        self.blocks[bounds.index(pos)] = val;
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
