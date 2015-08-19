use std::collections::HashMap;

use types::*;

use world::Plane;
use world::{Fragment, Hooks};
use world::ops::{self, OpResult};


pub fn create<'d, F>(f: &mut F, name: String) -> OpResult<PlaneId>
        where F: Fragment<'d> {
    let p = Plane {
        name: name,

        loaded_chunks: HashMap::new(),
        saved_chunks: HashMap::new(),

        stable_id: NO_STABLE_ID,
    };

    let pid = unwrap!(f.world_mut().planes.insert(p));
    post_init(f, pid);
    f.with_hooks(|h| h.on_plane_create(pid));
    Ok(pid)
}

pub fn create_unchecked<'d, F>(f: &mut F) -> PlaneId
        where F: Fragment<'d> {
    let pid = f.world_mut().planes.insert(Plane {
        name: String::new(),

        loaded_chunks: HashMap::new(),
        saved_chunks: HashMap::new(),

        stable_id: NO_STABLE_ID,
    }).unwrap();     // Shouldn't fail when stable_id == NO_STABLE_ID
    pid
}

pub fn post_init<'d, F>(f: &mut F,
                        pid: PlaneId)
        where F: Fragment<'d> {
    trace!("post_init({:?})", pid);
    let stable_pid = f.world_mut().planes.pin(pid);

    // limbo_entities is manipulated using multimap_insert/multimap_remove, so if there are no
    // entities in limbo targeting this plane, there may be not HashSet under the given key.
    if let Some(eids) = f.world_mut().limbo_entities.remove(&stable_pid) {
        let mut eids_vec = Vec::with_capacity(eids.len());
        for &eid in eids.iter() {
            f.world_mut().entities[eid].plane = pid;
            eids_vec.push(eid);
        }
        f.world_mut().entities_by_plane.insert(pid, eids);

        trace!("post_init: transfer {} entities for {:?} ({:?})", eids_vec.len(), pid, stable_pid);
        trace!("post_init: entities: {:?}", eids_vec);

        // TODO: Not sure this is a good idea.  The hook might mutate the world during this loop.
        for eid in eids_vec.into_iter() {
            f.with_hooks(|h| h.on_entity_plane_change(eid));
        }
    }
}

pub fn pre_fini<'d, F>(f: &mut F,
                       pid: PlaneId)
        where F: Fragment<'d> {
    trace!("pre_fini({:?})", pid);
    let stable_pid = f.world_mut().planes.pin(pid);

    // Same multimap_* stuff as is post_init.
    if let Some(eids) = f.world_mut().entities_by_plane.remove(&pid) {
        let mut eids_vec = Vec::with_capacity(eids.len());
        for &eid in eids.iter() {
            f.world_mut().entities[eid].plane = PLANE_LIMBO;
            eids_vec.push(eid);
        }
        f.world_mut().limbo_entities.insert(stable_pid, eids);

        // TODO: Not sure this is a good idea.  The hook might mutate the world during this loop.
        for eid in eids_vec.into_iter() {
            f.with_hooks(|h| h.on_entity_plane_change(eid));
        }
    }
}

pub fn destroy<'d, F>(f: &mut F,
                      pid: PlaneId) -> OpResult<()>
        where F: Fragment<'d> {
    pre_fini(f, pid);
    let p = unwrap!(f.world_mut().planes.remove(pid));

    for &tcid in p.loaded_chunks.values() {
        ops::terrain_chunk::destroy(f, tcid).unwrap();
    }

    f.with_hooks(|h| h.on_plane_destroy(pid));
    Ok(())
}
