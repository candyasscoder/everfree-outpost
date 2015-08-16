use std::collections::HashSet;

use types::*;
use util::stable_id_map::NO_STABLE_ID;

use world::{TerrainChunk, TerrainChunkFlags};
use world::{Fragment, Hooks};
use world::flags;
use world::ops::{self, OpResult};


pub fn create<'d, F>(f: &mut F,
                     pid: PlaneId,
                     cpos: V2) -> OpResult<TerrainChunkId>
        where F: Fragment<'d> {
    let tc = TerrainChunk {
        plane: pid,
        cpos: cpos,
        blocks: Box::new(PLACEHOLDER_CHUNK),

        stable_id: NO_STABLE_ID,
        flags: flags::TC_GENERATION_PENDING,
        child_structures: HashSet::new(),
    };

    // unwrap() always succeeds because stable_id is NO_STABLE_ID.
    let tcid = f.world_mut().terrain_chunks.insert(tc).unwrap();
    post_init(f, tcid);
    f.with_hooks(|h| h.on_terrain_chunk_create(tcid));
    Ok(tcid)
}

pub fn create_unchecked<'d, F>(f: &mut F) -> TerrainChunkId
        where F: Fragment<'d> {
    let tcid = f.world_mut().terrain_chunks.insert(TerrainChunk {
        plane: PlaneId(0),
        cpos: scalar(0),
        blocks: Box::new(EMPTY_CHUNK),

        stable_id: NO_STABLE_ID,
        flags: TerrainChunkFlags::empty(),
        child_structures: HashSet::new(),
    }).unwrap();     // Shouldn't fail when stable_id == NO_STABLE_ID
    tcid
}

pub fn post_init<'d, F>(f: &mut F,
                        tcid: TerrainChunkId)
        where F: Fragment<'d> {
    let w = f.world_mut();
    let tc = &w.terrain_chunks[tcid];
    // TODO: error handling: check for duplicate entries with same cpos
    w.planes[tc.plane].loaded_chunks.insert(tc.cpos, tcid);
}

pub fn pre_fini<'d, F>(f: &mut F,
                       tcid: TerrainChunkId)
        where F: Fragment<'d> {
    let w = f.world_mut();
    let tc = &w.terrain_chunks[tcid];
    // Containing plane may be missing during recursive destruction.
    w.planes.get_mut(tc.plane)
     .map(|p| p.loaded_chunks.remove(&tc.cpos));
}

pub fn destroy<'d, F>(f: &mut F,
                      tcid: TerrainChunkId) -> OpResult<()>
        where F: Fragment<'d> {
    trace!("destroy {:?}", tcid);
    pre_fini(f, tcid);
    let tc = unwrap!(f.world_mut().terrain_chunks.remove(tcid));

    for &sid in tc.child_structures.iter() {
        ops::structure::destroy(f, sid).unwrap();
    }

    f.with_hooks(|h| h.on_terrain_chunk_destroy(tcid, tc.plane, tc.cpos));
    Ok(())
}
