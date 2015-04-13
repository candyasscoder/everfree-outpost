use std::collections::HashSet;

use types::*;

use world::TerrainChunk;
use world::{Fragment, Hooks};
use world::ops::{self, OpResult};


pub fn create<'d, F>(f: &mut F,
                     pos: V2,
                     blocks: Box<BlockChunk>) -> OpResult<()>
        where F: Fragment<'d> {
    if f.world().terrain_chunks.contains_key(&pos) {
        fail!("chunk already exists with same position");
    }

    let tc = TerrainChunk {
        blocks: blocks,

        child_structures: HashSet::new(),
    };

    f.world_mut().terrain_chunks.insert(pos, tc);
    f.with_hooks(|h| h.on_terrain_chunk_create(pos));
    Ok(())
}

pub fn destroy<'d, F>(f: &mut F,
                      pos: V2) -> OpResult<()>
        where F: Fragment<'d> {
    let t = unwrap!(f.world_mut().terrain_chunks.remove(&pos));

    for &sid in t.child_structures.iter() {
        ops::structure::destroy(f, sid).unwrap();
    }

    f.with_hooks(|h| h.on_terrain_chunk_destroy(pos));
    Ok(())
}
