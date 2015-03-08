use std::collections::HashMap;
use std::collections::hash_map::Entry::*;
use std::ops::{Deref, DerefMut};

use physics::CHUNK_BITS;

use data::Data;
use types::*;
use util::Cursor;
use util::StrError;
use world::{World, Update};
use world::object::*;

pub use chunks::TerrainCache;


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

    // TODO: This type signature is super gross.  Should be possible to clean this up a bit by
    // making Cursor::new take any parent type that impls a certain (unsafe) trait, so that we can
    // let self_ be any 'CursorParent', instead of requiring precisely 'Cursor'.
    pub fn process_journal<P, S, F>(self_: Cursor<ManagedWorld<'d>, P, S>, mut f: F)
            where P: DerefMut,
                  S: Fn(&mut <P as Deref>::Target) -> &mut ManagedWorld<'d>,
                  F: FnMut(&mut Cursor<ManagedWorld<'d>, P, S>, Update) {
        World::process_journal(self_.extend(|mw| &mut mw.world), |w, u| {
            let mut mw = w.up();
            match u {
                Update::ChunkInvalidate(pos) => {
                    // Ignore errors.  The chunk might have been invalidated and then removed, for
                    // example.
                    let mw: &mut ManagedWorld = &mut **mw;
                    let _ = mw.cache.update(&mw.world, pos);
                },
                _ => {},
            }
            f(&mut *mw, u);
        });
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
            self.cache.update(&self.world, pos).unwrap();
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
