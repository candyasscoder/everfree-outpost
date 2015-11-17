//! Lifecycle management of terrain chunks.  This system tracks reference counts for all loaded
//! chunks.  External callers can request or release a particular chunk using the `load` and
//! `unload` methods.  (These methods don't actually load or unload except when the reference count
//! becomes non-/zero.)  The system itself only tracks reference counts - it relies on hooks
//! (called a `Provider`) to do the actual loading and unloading.
//!
//! In the overall architecture, some pieces of `logic` will load and unload chunks based on player
//! viewports, so that a certain amount of terrain surrounding each player is always loaded.  The
//! `Provider` invokes either savefile handling or terrain generation to load/unload.
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
    plane_ref_count: HashMap<PlaneId, u32>,
}

impl<'d> Chunks<'d> {
    pub fn new(storage: &'d Storage) -> Chunks<'d> {
        Chunks {
            storage: storage,
            lifecycle: Lifecycle::new(),
            plane_ref_count: HashMap::new(),
        }
    }
}

pub trait Provider {
    type E: Error;

    fn load_plane(&mut self, stable_pid: Stable<PlaneId>) -> Result<(), Self::E>;
    fn unload_plane(&mut self, pid: PlaneId) -> Result<(), Self::E>;

    fn load_terrain_chunk(&mut self, pid: PlaneId, cpos: V2) -> Result<(), Self::E>;
    fn unload_terrain_chunk(&mut self, pid: PlaneId, cpos: V2) -> Result<(), Self::E>;
}

// TODO: error handling in here is pretty bad (lots of warn_on_err)
pub trait Fragment<'d> {
    fn with_world<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Chunks<'d>, &World<'d>) -> R;

    type P: Provider;
    fn with_provider<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Chunks<'d>, &mut Self::P) -> R;

    fn get_plane_id(&mut self, stable_pid: Stable<PlaneId>) -> PlaneId {
        if let Some(pid) = self.with_world(|_, w| w.transient_plane_id(stable_pid)) {
            trace!("get_plane_id({:?}) = {:?} (hit)", stable_pid, pid);
            return pid;
        }

        self.with_provider(|_sys, provider| {
            warn_on_err!(provider.load_plane(stable_pid))
        });
        // Correctly implemented provider should create or load a Plane with the given StableId.
        let pid = self.with_world(|_, w| w.transient_plane_id(stable_pid)).unwrap();
        trace!("get_plane_id({:?}) = {:?} (miss)", stable_pid, pid);
        pid
    }

    fn unload_plane(&mut self, pid: PlaneId) {
        self.with_provider(|sys, provider| {
            if let Some(&ref_count) = sys.plane_ref_count.get(&pid) {
                if ref_count > 0 {
                    warn!("unloading {:?} despite nonzero ref count", pid);
                }
            }

            warn_on_err!(provider.unload_plane(pid));
        });
    }

    /// Returns `true` iff the chunk was actually loaded as a result of this call (as opposed to
    /// simply having its refcount incremented).
    fn load(&mut self, pid: PlaneId, cpos: V2) -> bool {
        trace!("load({:?}, {:?})", pid, cpos);
        self.with_provider(|sys, provider| {
            let first = sys.lifecycle.retain(pid, cpos, |pid, cpos| {
                warn_on_err!(provider.load_terrain_chunk(pid, cpos))
            });
            if first {
                // No need to load anything, since the Plane must already be loaded to have a
                // PlaneId.
                match sys.plane_ref_count.entry(pid) {
                    Vacant(e) => { e.insert(1); },
                    Occupied(e) => { *e.into_mut() += 1; },
                }
            }
            first
        })
    }

    /// Returns `true` iff the chunk was actually unloaded as a result of this call.
    fn unload(&mut self, pid: PlaneId, cpos: V2) -> bool {
        trace!("unload({:?}, {:?})", pid, cpos);
        self.with_provider(|sys, provider| {
            let last = sys.lifecycle.release(pid, cpos, |pid, cpos| {
                warn_on_err!(provider.unload_terrain_chunk(pid, cpos))
            });
            if last {
                if let Occupied(mut e) = sys.plane_ref_count.entry(pid) {
                    *e.get_mut() -= 1;
                    if *e.get() == 0 {
                        e.remove();
                        warn_on_err!(provider.unload_plane(pid));
                    }
                } else {
                    panic!("tried to release plane {:?}, but its ref_count is already 0",
                           pid);
                }
            }
            last
        })
    }
}


struct Lifecycle {
    // When client code requests chunk (x,y), we load not only (x,y) but also (x-1,y), (x,y-1), and
    // (x-1,y-1).  This ensures that every structure overlapping (x,y) is loaded, even those whose
    // base lies outside the chunk itself.
    //
    // To keep track of these internal references, we use two different reference counts.
    // `user_ref_count` counts the number of external users that have requested to load the chunk.
    // `ref_count` counts "internal users", which are the four nearby chunks that care about
    // structures here.
    ref_count: HashMap<(PlaneId, V2), u32>,
    user_ref_count: HashMap<(PlaneId, V2), u32>,
}

impl Lifecycle {
    pub fn new() -> Lifecycle {
        Lifecycle {
            ref_count: HashMap::new(),
            user_ref_count: HashMap::new(),
        }
    }

    pub fn retain<F>(&mut self,
                     pid: PlaneId,
                     cpos: V2,
                     mut load: F) -> bool
            where F: FnMut(PlaneId, V2) {
        let first = match self.user_ref_count.entry((pid, cpos)) {
            Vacant(e) => {
                e.insert(1);
                debug!("retain: 1 users of {:?} {:?}", pid, cpos);
                true
            },
            Occupied(e) => {
                debug!("retain: {} users of {:?} {:?}", 1 + *e.get(), pid, cpos);
                *e.into_mut() += 1;
                false
            },
        };

        if first {
            for subpos in Region::around(cpos, 1).points() {
                self.retain_inner(pid, subpos, &mut load);
            }
        }

        first
    }

    pub fn release<F>(&mut self,
                      pid: PlaneId,
                      cpos: V2,
                      mut unload: F) -> bool
            where F: FnMut(PlaneId, V2) {
        let last = if let Occupied(mut e) = self.user_ref_count.entry((pid, cpos)) {
            *e.get_mut() -= 1;
            debug!("release: {} users of {:?} {:?}", *e.get(), pid, cpos);
            if *e.get() == 0 {
                e.remove();
                true
            } else {
                false
            }
        } else {
            panic!("tried to release chunk {:?} {:?}, but its user_ref_count is already zero",
                   pid, cpos);
        };

        if last {
            for subpos in Region::around(cpos, 1).points() {
                self.release_inner(pid, subpos, &mut unload);
            }
        }

        last
    }

    pub fn retain_inner<F>(&mut self,
                           pid: PlaneId,
                           cpos: V2,
                           load: &mut F)
            where F: FnMut(PlaneId, V2) {
        let first = match self.ref_count.entry((pid, cpos)) {
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
            (*load)(pid, cpos);
        }
    }

    pub fn release_inner<F>(&mut self,
                            pid: PlaneId,
                            cpos: V2,
                            unload: &mut F)
            where F: FnMut(PlaneId, V2) {
        let last = if let Occupied(mut e) = self.ref_count.entry((pid, cpos)) {
            *e.get_mut() -= 1;
            if *e.get() == 0 {
                e.remove();
                true
            } else {
                false
            }
        } else {
            panic!("tried to release chunk {:?} {:?}, but its ref_count is already zero",
                   pid, cpos);
        };

        if last {
            (*unload)(pid, cpos);
        }
    }
}
