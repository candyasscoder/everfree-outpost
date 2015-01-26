use std::collections::HashMap;
use std::collections::hash_map::Entry::*;

use physics::{CHUNK_SIZE, CHUNK_BITS};
use physics::v3::{Vn, V3, V2, scalar, Region};

use data::Data;
use types::*;
use util::StrError;
use world::{World, Update};
use world::object::*;

pub struct TerrainCache {
    cache: HashMap<V2, CacheEntry>,
}

struct CacheEntry {
    blocks: [BlockId; 1 << (3 * CHUNK_BITS)],
}

impl TerrainCache {
    pub fn new() -> TerrainCache {
        TerrainCache {
            cache: HashMap::new(),
        }
    }

    pub fn update(&mut self, w: &World, chunk_pos: V2) -> Result<(), StrError> {
        let chunk = unwrap!(w.get_terrain_chunk(chunk_pos));
        let mut entry = CacheEntry::new(*chunk.blocks());

        let chunk_bounds = chunk.bounds();
        for s in w.chunk_structures(chunk_pos) {
            let struct_bounds = s.bounds();
            let t = s.template();
            for point in struct_bounds.intersect(chunk_bounds).points() {
                let block = t.blocks[struct_bounds.index(point)];
                entry.blocks[chunk_bounds.index(point)] = block;
            }
        }

        match self.cache.entry(chunk_pos) {
            Vacant(e) => { e.insert(entry); },
            Occupied(e) => { *e.into_mut() = entry; },
        }

        Ok(())
    }

    pub fn forget(&mut self, chunk_pos: V2) {
        self.cache.remove(&chunk_pos);
    }

    pub fn get(&self, chunk_pos: V2) -> Option<&BlockChunk> {
        self.cache.get(&chunk_pos).map(|c| &c.blocks)
    }
}

impl CacheEntry {
    pub fn new(blocks: [BlockId; 1 << (3 * CHUNK_BITS)]) -> CacheEntry {
        CacheEntry {
            blocks: blocks,
        }
    }
}


pub struct ManagedWorld<'d> {
    world: World<'d>,
    cache: TerrainCache,

    ref_count: HashMap<V2, usize>,
    structure_ref_count: HashMap<V2, usize>,
}

impl<'d> ManagedWorld<'d> {
    pub fn new(data: &'d Data) -> ManagedWorld<'d> {
        ManagedWorld {
            world: World::new(data),
            cache: TerrainCache::new(),

            ref_count: HashMap::new(),
            structure_ref_count: HashMap::new(),
        }
    }

    pub fn world(&self) -> &World<'d> {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World<'d> {
        &mut self.world
    }

    pub fn refresh_chunk(&mut self, chunk_pos: V2) {
        self.cache.update(&self.world, chunk_pos).unwrap();
    }

    pub fn get_terrain(&self, chunk_pos: V2) -> Option<&BlockChunk> {
        self.cache.get(chunk_pos)
    }

    pub fn retain<F1, F2>(&mut self,
                          pos: V2,
                          mut load_terrain: F1,
                          mut load_objects: F2)
            where F1: FnMut(V2) -> BlockChunk,
                  F2: FnMut(V2) -> Vec<(V3, TemplateId)> {
        match self.ref_count.entry(pos) {
            Vacant(e) => {
                self.world.create_terrain_chunk(pos, load_terrain(pos)).unwrap();
                e.insert(1);

                for subpos in Region::around(pos, 1).points() {
                    retain_structures(&mut self.world,
                                      &mut self.structure_ref_count,
                                      subpos,
                                      |x| load_objects(x));
                }

                self.cache.update(&self.world, pos);
            },
            Occupied(e) => {
                *e.into_mut() += 1;
            },
        }
    }

    pub fn release(&mut self,
                   pos: V2) {
        if let Occupied(mut e) = self.ref_count.entry(pos) {
            *e.get_mut() -= 1;

            if *e.get() == 0 {
                self.cache.forget(pos);

                for subpos in Region::around(pos, 1).points() {
                    release_structures(&mut self.world,
                                       &mut self.structure_ref_count,
                                       subpos);
                }

                e.remove();
                self.world.destroy_terrain_chunk(pos).unwrap();
            }
        } else {
            panic!("tried to release non-loaded chunk {:?}", pos);
        }
    }
}

fn retain_structures<F>(world: &mut World,
                        structure_ref_count: &mut HashMap<V2, usize>,
                        pos: V2,
                        mut load_structures: F)
        where F: FnMut(V2) -> Vec<(V3, TemplateId)> {
    match structure_ref_count.entry(pos) {
        Vacant(e) => {
            for (p, tid) in load_structures(pos).into_iter() {
                // TODO: check that pos is valid for the chunk
                world.create_structure(p, tid); //.unwrap();
            }
            e.insert(1);
        },
        Occupied(e) => {
            *e.into_mut() += 1;
        },
    }
}

fn release_structures(world: &mut World,
                      structure_ref_count: &mut HashMap<V2, usize>,
                      pos: V2) {
    if let Occupied(mut e) = structure_ref_count.entry(pos) {
        *e.get_mut() -= 1;

        if *e.get() == 0 {
            let base = pos.extend(0) * scalar(CHUNK_SIZE);
            let bounds = Region::new(base, base + scalar(CHUNK_SIZE));
            let sids = world.chunk_structures(pos)
                            .filter(|s| bounds.contains(s.pos()))
                            .map(|s| s.id())
                            .collect::<Vec<_>>();
            for sid in sids.into_iter() {
                world.destroy_structure(sid).unwrap();
            }

            e.remove();
        }
    } else {
        panic!("tried to release non-loaded structs {:?}", pos);
    }
}
