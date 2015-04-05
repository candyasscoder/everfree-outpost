use std::collections::HashMap;

use types::*;
use util::StrResult;
use physics::{CHUNK_BITS, CHUNK_SIZE, Shape};

use world::World;
use world::object::*;


pub struct TerrainCache {
    cache: HashMap<V2, CacheEntry>,
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

    pub fn add_chunk(&mut self, w: &World, cpos: V2) -> StrResult<()> {
        info!("add chunk {:?}", cpos);
        let mut entry = CacheEntry::new();

        let base = cpos.extend(0) * scalar(CHUNK_SIZE);
        let bounds = Region::new(base, base + scalar(CHUNK_SIZE));
        try!(compute_shape(w, cpos, bounds, &mut entry));

        self.cache.insert(cpos, entry);
        Ok(())
    }

    pub fn remove_chunk(&mut self, cpos: V2) {
        info!("remove chunk {:?}", cpos);
        self.cache.remove(&cpos);
    }

    pub fn update_region(&mut self, w: &World, bounds: Region) {
        for cpos in bounds.reduce().div_round_signed(CHUNK_SIZE).points() {
            info!("update chunk {:?}", cpos);
            if let Some(entry) = self.cache.get_mut(&cpos) {
                compute_shape(w, cpos, bounds, entry);
            }
        }
    }

    pub fn get(&self, cpos: V2) -> Option<&CacheEntry> {
        self.cache.get(&cpos)
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
                 cpos: V2,
                 bounds: Region,
                 entry: &mut CacheEntry) -> StrResult<()> {
    let data = w.data();
    let chunk = unwrap!(w.get_terrain_chunk(cpos));
    let bounds = bounds.intersect(chunk.bounds());

    for p in bounds.points() {
        let idx = chunk.bounds().index(p);
        entry.shape[idx] = data.block_data.shape(chunk.block(idx));
    }

    for s in w.chunk_structures(cpos) {
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
