use std::collections::HashSet;

use types::*;
use util::stable_id_map::NO_STABLE_ID;

use world::TerrainChunk;
use world::{Fragment, Hooks};
use world::ops::{self, OpResult};


pub fn create<'d, F>(f: &mut F,
                     pid: PlaneId,
                     cpos: V2,
                     blocks: Box<BlockChunk>) -> OpResult<TerrainChunkId>
        where F: Fragment<'d> {
    let tc = TerrainChunk {
        plane: pid,
        cpos: cpos,
        blocks: blocks,

        stable_id: NO_STABLE_ID,
        child_structures: HashSet::new(),
    };

    // unwrap() always succeeds because stable_id is NO_STABLE_ID.
    let tcid = f.world_mut().terrain_chunks.insert(tc).unwrap();
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
        child_structures: HashSet::new(),
    }).unwrap();     // Shouldn't fail when stable_id == NO_STABLE_ID
    tcid
}

pub fn destroy<'d, F>(f: &mut F,
                      tcid: TerrainChunkId) -> OpResult<()>
        where F: Fragment<'d> {
    let tc = unwrap!(f.world_mut().terrain_chunks.remove(tcid));

    f.world_mut().planes[tc.plane].loaded_chunks.remove(&tc.cpos);

    for &sid in tc.child_structures.iter() {
        ops::structure::destroy(f, sid).unwrap();
    }

    f.with_hooks(|h| h.on_terrain_chunk_destroy(tcid));
    Ok(())
}
