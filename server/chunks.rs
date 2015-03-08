use std::collections::HashMap;
use std::collections::hash_map::Entry::*;
use std::error::Error;

use physics::CHUNK_BITS;
use types::*;
use util::StrResult;

use script::ScriptEngine;
use storage::Storage;
use world::World;
use world::object::*;


pub struct Chunks<'d> {
    storage: &'d Storage,

    cache: TerrainCache,
    lifecycle: Lifecycle,
}

impl<'d> Chunks<'d> {
    pub fn new(storage: &'d Storage) -> Chunks<'d> {
        Chunks {
            storage: storage,
            cache: TerrainCache::new(),
            lifecycle: Lifecycle::new(),
        }
    }
}

pub trait Hooks {
    fn post_load(&mut self, chunk_pos: V2);
    fn pre_unload(&mut self, chunk_pos: V2);
}

pub trait Provider {
    type E: Error;
    fn load(&mut self, cpos: V2) -> Result<(), Self::E>;
    fn unload(&mut self, cpos: V2) -> Result<(), Self::E>;
}

pub trait Fragment<'d> {
    fn world(&mut self) -> (&mut Chunks<'d>, &World<'d>);

    type H: Hooks;
    fn hooks(&mut self) -> &mut Self::H;

    type P: Provider;
    fn provider(&mut self) -> (&mut Chunks<'d>, &mut Self::P);

    fn load(&mut self, cpos: V2) {
        {
            let (sys, provider) = self.provider();
            sys.lifecycle.retain(cpos, |cpos| warn_on_err!(provider.load(cpos)));
        }
        {
            let (sys, world) = self.world();
            warn_on_err!(sys.cache.update(world, cpos));
        }
        self.hooks().post_load(cpos);
    }

    fn unload(&mut self, cpos: V2) {
        self.hooks().pre_unload(cpos);
        {
            let (sys, world) = self.world();
            sys.cache.forget(cpos);
        }
        {
            let (sys, provider) = self.provider();
            sys.lifecycle.release(cpos, |cpos| warn_on_err!(provider.unload(cpos)));
        }
    }

    fn update(&mut self, cpos: V2) -> StrResult<()> {
        let (sys, world) = self.world();
        sys.cache.update_if_present(world, cpos)
    }
}


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

    pub fn update(&mut self, w: &World, chunk_pos: V2) -> StrResult<()> {
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

    pub fn contains(&self, chunk_pos: V2) -> bool {
        self.cache.contains_key(&chunk_pos)
    }

    pub fn update_if_present(&mut self, w: &World, chunk_pos: V2) -> StrResult<()> {
        if self.contains(chunk_pos) {
            try!(self.update(w, chunk_pos));
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


struct Lifecycle {
    // Keep two separate refcounts for each chunk.  We do this to deal with the fact that building
    // the cached terrain for a chunk requires access not only to that chunk but also to its three
    // neighbors to the north and west.  `ref_count > 0` means the chunk is loaded for some reason.
    // `user_ref_count > 0` means the chunk is loaded because some external user wants the cached
    // terrain to be availaible (so the chunk and its three neighbors must all be loaded).
    ref_count: HashMap<V2, u32>,
    user_ref_count: HashMap<V2, u32>,
}

impl Lifecycle {
    pub fn new() -> Lifecycle {
        Lifecycle {
            ref_count: HashMap::new(),
            user_ref_count: HashMap::new(),
        }
    }

    pub fn retain<F>(&mut self,
                     pos: V2,
                     mut load: F)
            where F: FnMut(V2) {
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
        }
    }

    pub fn release<F>(&mut self,
                      pos: V2,
                      mut unload: F)
            where F: FnMut(V2) {
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
        }
    }

    pub fn retain_inner<F>(&mut self,
                           pos: V2,
                           load: &mut F)
            where F: FnMut(V2) {
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
            (*load)(pos);
        }
    }

    pub fn release_inner<F>(&mut self,
                            pos: V2,
                            unload: &mut F)
            where F: FnMut(V2) {
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
            (*unload)(pos);
        }
    }
}
