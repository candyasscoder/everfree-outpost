use std::collections::HashMap;
use std::hash::{Hash, Hasher, SipHasher};
use std::mem;
use std::thread::{self, JoinGuard};
use std::sync::mpsc::{self, Sender, Receiver};
use rand::{Rng, XorShiftRng, SeedableRng};

use libphysics::CHUNK_SIZE;
use types::*;
use util::StrResult;

use data::Data;
use engine::split::PartFlags;
use script::ScriptEngine;
use storage::Storage;
use world::Fragment as World_Fragment;
use world::Hooks;
use world::StructureAttachment;
use world::flags;
use world::object::*;

mod worker;
mod prop;
mod cache;

mod dsc;
mod cellular;
mod disk_sampler2;
mod pattern;

mod forest;


pub type StdRng = XorShiftRng;


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
            let pid = unwrap_or!(wf.world().transient_plane_id(stable_pid));

            let tcid = {
                let mut p = wf.plane_mut(pid);
                let mut tc = unwrap_or!(p.get_terrain_chunk_mut(cpos));
 
                if !tc.flags().contains(flags::TC_GENERATION_PENDING) {
                    // Prevent this:
                    //  1) Load chunk, start generating
                    //  2) Unload chunk (but keep generating from #1)
                    //  3) Load chunk, start generating (queued, #1 is still going)
                    //  4) Generation #1 finishes; chunk is loaded so set its contents
                    //  5) Player modifies chunk
                    //  6) Generation #3 finishes; RESET chunk contents (erasing modifications)
                    return;
                }

                *tc.blocks_mut() = *gc.blocks;
                tc.flags_mut().remove(flags::TC_GENERATION_PENDING);
                tc.id()
            };
            wf.with_hooks(|h| h.on_terrain_chunk_update(tcid));

            let base = cpos.extend(0) * scalar(CHUNK_SIZE);
            for gs in &gc.structures {
                let sid = match wf.create_structure_unchecked(pid,
                                                              base + gs.pos,
                                                              gs.template) {
                    Ok(mut s) => {
                        s.set_attachment(StructureAttachment::Chunk);
                        s.id()
                    },
                    Err(e) => {
                        warn!("error placing generated structure: {}",
                              ::std::error::Error::description(&e));
                        continue;
                    },
                };
                for (k, v) in &gs.extra {
                    warn_on_err!(ScriptEngine::cb_apply_structure_extra(wf, sid, k, v));
                }
            }
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
