use std::collections::HashMap;
use std::collections::hash_map::Entry::*;

use physics::{CHUNK_SIZE, CHUNK_BITS};
use physics::v3::{Vn, V3, V2, scalar, Region};

use data::Data;
use types::*;
use util::StrError;
use world::{World, Update};
use world::StructureAttachment;
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

    // Keep two separate refcounts for each chunk.  We do this to deal with the fact that building
    // the cached terrain for a chunk requires access not only to that chunk but also to its three
    // neighbors to the north and west.  `ref_count > 0` means the chunk is loaded for some reason.
    // `user_ref_count > 0` means the chunk is loaded because some external user wants the cached
    // terrain to be availaible (so the chunk and its three neighbors must all be loaded).
    ref_count: HashMap<V2, u32>,
    user_ref_count: HashMap<V2, u32>,
}

impl<'d> ManagedWorld<'d> {
    pub fn new(data: &'d Data) -> ManagedWorld<'d> {
        ManagedWorld {
            world: World::new(data),
            cache: TerrainCache::new(),

            ref_count: HashMap::new(),
            user_ref_count: HashMap::new(),
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

    pub fn retain<F>(&mut self,
                     pos: V2,
                     mut load: F)
            where F: FnMut(&mut World, V2) {
        let first = match self.user_ref_count.entry(pos) {
            Vacant(e) => {
                e.insert(1);
                true
            },
            Occupied(e) => {
                *e.into_mut() += 1;
                false
            },
        };

        if first {
            for subpos in Region::around(pos, 1).points() {
                self.retain_inner(subpos, &mut load);
            }
            self.cache.update(&self.world, pos);
        }
    }

    pub fn release<F>(&mut self,
                      pos: V2,
                      mut unload: F)
            where F: FnMut(&mut World, V2) {
        let last = if let Occupied(mut e) = self.user_ref_count.entry(pos) {
            *e.get_mut() -= 1;
            if *e.get() == 0 {
                e.remove();
                true
            } else {
                false
            }
        } else {
            panic!("tried to release chunk {:?}, but its user_ref_count is already zero", pos);
        };

        if last {
            for subpos in Region::around(pos, 1).points() {
                self.release_inner(subpos, &mut unload);
            }
            self.cache.forget(pos);
        }
    }

    pub fn retain_inner<F>(&mut self,
                           pos: V2,
                           load: &mut F)
            where F: FnMut(&mut World, V2) {
        let first = match self.ref_count.entry(pos) {
            Vacant(e) => {
                e.insert(1);
                true
            },
            Occupied(e) => {
                *e.into_mut() += 1;
                false
            }
        };

        if first {
            (*load)(&mut self.world, pos);
            assert!(self.world.get_terrain_chunk(pos).is_some());
        }
    }

    pub fn release_inner<F>(&mut self,
                            pos: V2,
                            unload: &mut F)
            where F: FnMut(&mut World, V2) {
        let last = if let Occupied(mut e) = self.ref_count.entry(pos) {
            *e.get_mut() -= 1;
            if *e.get() == 0 {
                e.remove();
                true
            } else {
                false
            }
        } else {
            panic!("tried to release chunk {:?}, but its ref_count is already zero", pos);
        };

        if last {
            (*unload)(&mut self.world, pos);
        }
    }
}
