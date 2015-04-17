use std::collections::HashMap;

use types::*;
use util::stable_id_map::NO_STABLE_ID;

use world::Plane;
use world::{Fragment, Hooks};
use world::ops::{self, OpResult};


pub fn create<'d, F>(f: &mut F) -> OpResult<PlaneId>
        where F: Fragment<'d> {
    let pid = create_unchecked(f);
    f.with_hooks(|h| h.on_plane_create(pid));
    Ok(pid)
}

pub fn create_unchecked<'d, F>(f: &mut F) -> PlaneId
        where F: Fragment<'d> {
    let pid = f.world_mut().planes.insert(Plane {
        loaded_chunks: HashMap::new(),
        saved_chunks: HashMap::new(),

        stable_id: NO_STABLE_ID,
    }).unwrap();     // Shouldn't fail when stable_id == NO_STABLE_ID
    pid
}

pub fn destroy<'d, F>(f: &mut F,
                      pid: PlaneId) -> OpResult<()>
        where F: Fragment<'d> {
    let p = unwrap!(f.world_mut().planes.remove(pid));

    for &tcid in p.loaded_chunks.values() {
        ops::terrain_chunk::destroy(f, tcid).unwrap();
    }

    f.with_hooks(|h| h.on_plane_destroy(pid));
    Ok(())
}
