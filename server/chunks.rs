use std::collections::HashMap;
use std::collections::hash_map::Entry::*;
use std::error::Error;

use types::*;

use storage::Storage;
use world::World;
use world::object::*;


pub struct Chunks<'d> {
    storage: &'d Storage,

    lifecycle: Lifecycle,
}

impl<'d> Chunks<'d> {
    pub fn new(storage: &'d Storage) -> Chunks<'d> {
        Chunks {
            storage: storage,
            lifecycle: Lifecycle::new(),
        }
    }
}

#[allow(unused_variables)]
pub trait Hooks {
    fn post_load(&mut self, cpos: V2) {}
    fn pre_unload(&mut self, cpos: V2) {}
}

pub trait Provider {
    type E: Error;
    fn load(&mut self, cpos: V2) -> Result<(), Self::E>;
    fn unload(&mut self, cpos: V2) -> Result<(), Self::E>;
}

pub trait Fragment<'d> {
    fn with_world<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Chunks<'d>, &World<'d>) -> R;

    type H: Hooks;
    fn with_hooks<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Self::H) -> R;

    type P: Provider;
    fn with_provider<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Chunks<'d>, &mut Self::P) -> R;

    /// Returns `true` iff the chunk was actually loaded as a result of this call (as opposed to
    /// simply having its refcount incremented).
    fn load(&mut self, cpos: V2) -> bool {
        let first = self.with_provider(|sys, provider| {
            sys.lifecycle.retain(cpos, |cpos| warn_on_err!(provider.load(cpos)))
        });
        self.with_hooks(|hooks| hooks.post_load(cpos));
        first
    }

    /// Returns `true` iff the chunk was actually unloaded as a result of this call.
    fn unload(&mut self, cpos: V2) -> bool {
        self.with_hooks(|hooks| hooks.pre_unload(cpos));
        let last = self.with_provider(|sys, provider| {
            sys.lifecycle.release(cpos, |cpos| warn_on_err!(provider.unload(cpos)))
        });
        last
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
                     mut load: F) -> bool
            where F: FnMut(V2) {
        let first = match self.user_ref_count.entry(pos) {
            Vacant(e) => {
                e.insert(1);
                debug!("retain: 1 users of {:?}", pos);
                true
            },
            Occupied(e) => {
                debug!("retain: {} users of {:?}", 1 + *e.get(), pos);
                *e.into_mut() += 1;
                false
            },
        };

        if first {
            for subpos in Region::around(pos, 1).points() {
                self.retain_inner(subpos, &mut load);
            }
        }

        first
    }

    pub fn release<F>(&mut self,
                      pos: V2,
                      mut unload: F) -> bool
            where F: FnMut(V2) {
        let last = if let Occupied(mut e) = self.user_ref_count.entry(pos) {
            *e.get_mut() -= 1;
            debug!("release: {} users of {:?}", *e.get(), pos);
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

        last
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
