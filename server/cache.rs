use std::collections::HashMap;

use types::*;
use util::StrResult;
use physics::{CHUNK_BITS, CHUNK_SIZE, Shape};

use world::World;
use world::object::*;


pub struct TerrainCache {
    cache: HashMap<(PlaneId, V2), CacheEntry>,
}

struct CacheEntry {
    pub shape: [Shape; 1 << (3 * CHUNK_BITS)],
    pub layer_mask: [u8; 1 << (3 * CHUNK_BITS)],
}

impl TerrainCache {
    pub fn new() -> TerrainCache {
        TerrainCache {
            cache: HashMap::new(),
        }
    }

    pub fn add_chunk(&mut self, w: &World, pid: PlaneId, cpos: V2) -> StrResult<()> {
        let mut entry = CacheEntry::new();

        let base = cpos.extend(0) * scalar(CHUNK_SIZE);
        let bounds = Region::new(base, base + scalar(CHUNK_SIZE));
        try!(compute_shape(w, pid, cpos, bounds, &mut entry));

        self.cache.insert((pid, cpos), entry);
        Ok(())
    }

    pub fn remove_chunk(&mut self, pid: PlaneId, cpos: V2) {
        self.cache.remove(&(pid, cpos));
    }

    pub fn update_region(&mut self, w: &World, pid: PlaneId, bounds: Region) {
        for cpos in bounds.reduce().div_round_signed(CHUNK_SIZE).points() {
            if let Some(entry) = self.cache.get_mut(&(pid, cpos)) {
                // Fails only when the plane/terrain chunk does not exist.  Since the cache entry
                // exists, the plane/chunk should also exist.
                compute_shape(w, pid, cpos, bounds, entry).unwrap();
            }
        }
    }

    pub fn get(&self, pid: PlaneId, cpos: V2) -> Option<&CacheEntry> {
        self.cache.get(&(pid, cpos))
    }
}

impl CacheEntry {
    pub fn new() -> CacheEntry {
        CacheEntry {
            shape: [Shape::Empty; 1 << (3 * CHUNK_BITS)],
            layer_mask: [0; 1 << (3 * CHUNK_BITS)],
        }
    }
}


fn compute_shape(w: &World,
                 pid: PlaneId,
                 cpos: V2,
                 bounds: Region,
                 entry: &mut CacheEntry) -> StrResult<()> {
    trace!("compute_shape({:?}, {:?})", pid, cpos);
    let data = w.data();
    let p = unwrap!(w.get_plane(pid));
    let chunk = unwrap!(p.get_terrain_chunk(cpos));
    let bounds = bounds.intersect(chunk.bounds());

    for p in bounds.points() {
        let idx = chunk.bounds().index(p);
        entry.shape[idx] = data.block_data.shape(chunk.block(idx));
        entry.layer_mask[idx] = 0;
    }

    for s in w.chunk_structures(pid, cpos) {
        if !s.bounds().overlaps(bounds) {
            continue;
        }

        for p in s.bounds().intersect(bounds).points() {
            let template = s.template();
            let s_idx = s.bounds().index(p);
            let c_idx = chunk.bounds().index(p);
            if shape_overrides(entry.shape[c_idx], template.shape[s_idx]) {
                entry.shape[c_idx] = template.shape[s_idx];
            }
            entry.layer_mask[c_idx] |= 1 << (template.layer as usize);
        }
    }

    Ok(())
}

fn shape_overrides(old: Shape, new: Shape) -> bool {
    match (old, new) {
        (Shape::Empty, _) => true,

        (Shape::Floor, Shape::Empty) => false,
        (Shape::Floor, _) => true,

        (Shape::Solid, _) => false,

        _ => false,
    }
}
