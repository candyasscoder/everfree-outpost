use std::collections::HashSet;

use physics::{CHUNK_SIZE, CHUNK_BITS};
use physics::v3::{V3, Region, scalar};

use data::Data;
use types::BlockId;
use util::RefcountedMap;


const CHUNK_TOTAL: usize = 1 << (3 * CHUNK_BITS);
pub type BlockChunk = [BlockId; CHUNK_TOTAL];
pub static EMPTY_CHUNK: BlockChunk = [0; CHUNK_TOTAL];


pub struct Object {
    pub template_id: u32,
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

#[derive(PartialEq, Eq, Show)]
enum CacheStatus {
    Uninitialized,
    Clean,
    Dirty,
}

struct ChunkTerrain {
    base: BlockChunk,
    cache: BlockChunk,
    cache_status: CacheStatus,
}

struct ChunkObjects {
    objects: Vec<Object>,
}

pub struct Terrain<'d> {
    data: &'d Data,
    terrain: RefcountedMap<(i32, i32), ChunkTerrain>,
    objects: RefcountedMap<(i32, i32), ChunkObjects>,
    dirty: HashSet<(i32, i32)>,
}

impl<'d> Terrain<'d> {
    pub fn new(data: &'d Data) -> Terrain<'d> {
        Terrain {
            data: data,
            terrain: RefcountedMap::new(),
            objects: RefcountedMap::new(),
            dirty: HashSet::new(),
        }
    }

    pub fn retain<F1, F2>(&mut self,
                          pos: (i32, i32),
                          mut load_terrain: F1,
                          mut load_objects: F2) -> &BlockChunk
            where F1: FnMut(i32, i32) -> BlockChunk,
                  F2: FnMut(i32, i32) -> Vec<Object> {
        let (x, y) = pos;

        let data = self.data;
        let terrain_map = &mut self.terrain;
        let objects_map = &mut self.objects;

        let (terrain, _) = terrain_map.retain((x, y), || {
            let terrain = load_terrain(x, y);
            let mut cache = terrain;

            build_cache(data,
                        |obj_x, obj_y| {
                            let (objs, _) = objects_map.retain((obj_x, obj_y), || {
                                ChunkObjects {
                                    objects: load_objects(obj_x, obj_y),
                                }
                            });
                            objs.objects.as_slice()
                        },
                        &mut cache,
                        (x, y));

            ChunkTerrain {
                base: terrain,
                cache: cache,
                cache_status: CacheStatus::Clean,
            }
        });

        &terrain.cache
    }

    pub fn release<F1, F2>(&mut self,
                           pos: (i32, i32),
                           mut evict_terrain: F1,
                           mut evict_objects: F2)
            where F1: FnMut(i32, i32, BlockChunk),
                  F2: FnMut(i32, i32, ChunkObjects) {
        let (x,y) = pos;

        let terrain_map = &mut self.terrain;
        let objects_map = &mut self.objects;

        terrain_map.release((x, y), |terrain| {
            evict_terrain(x, y, terrain.base);
            
            for dy in range(0, 2) {
                for dx in range(0, 2) {
                    let obj_x = x - dx;
                    let obj_y = y - dy;
                    objects_map.release((obj_x, obj_y), |objects| {
                        evict_objects(obj_x, obj_y, objects);
                    });
                }
            }
        });
    }

    pub fn get(&self, pos: (i32, i32)) -> &BlockChunk {
        match self.terrain.get(&pos) {
            Some(ref chunk) => {
                if chunk.cache_status != CacheStatus::Clean {
                    warn!("tried to access chunk {:?}, but cache was {:?}",
                          pos, chunk.cache_status);
                    &chunk.base
                } else {
                    &chunk.cache
                }
            },
            None => &EMPTY_CHUNK,
        }
    }

    fn find_object_at_point(&mut self,
                            point: V3) -> Option<(i32, i32, usize)> {
        let point_chunk = point.div_floor(&scalar(CHUNK_SIZE));

        // Look at objects in the chunk containing `point` and also in the chunks above and to the
        // left.
        for offset_y in range(0, 2) {
            for offset_x in range(0, 2) {
                let cx = point_chunk.x - offset_x;
                let cy = point_chunk.y - offset_y;

                let objs = match self.objects.get(&(cx, cy)) {
                    None => continue,
                    Some(x) => x,
                };

                let chunk_pos = V3::new(cx, cy, 0) * scalar(CHUNK_SIZE);

                for (idx, obj) in objs.objects.iter().enumerate() {
                    let base = chunk_pos + V3::new(obj.x as i32,
                                                   obj.y as i32,
                                                   obj.z as i32);
                    let template = self.data.object_templates.template(obj.template_id);
                    let region = Region::new(base, base + template.size);

                    if region.contains(&point) {
                        return Some((cx, cy, idx));
                    }
                }
            }
        }

        None
    }

    pub fn replace_object_at_point(&mut self,
                                   point: V3,
                                   new_id: u32) -> bool {
        let (cx, cy, idx) = match self.find_object_at_point(point) {
            None => return false,
            Some(x) => x,
        };

        let chunk_objs = match self.objects.get_mut(&(cx, cy)) {
            None => return false,
            Some(x) => x,
        };
        let objs = &mut chunk_objs.objects;
        let old_template = self.data.object_templates.template(objs[idx].template_id);
        let new_template = self.data.object_templates.template(new_id);

        let mut obj = &mut objs[idx];
        obj.template_id = new_id;

        let old_chunk_base = V3::new(cx, cy, 0) * scalar(CHUNK_SIZE);
        let old_pos = old_chunk_base + V3::new(obj.x as i32,
                                               obj.y as i32,
                                               obj.z as i32);
        let old_bounds = Region::new(old_pos, old_pos + old_template.size);
        let new_bounds = Region::new(old_pos, old_pos + new_template.size);

        // NB: When we start to allow objects to be moved, it might not be a good idea to use .join
        // (if the object moved a long distance, this would invalidate all intervening chunks).
        invalidate_region(old_bounds.join(&new_bounds),
                          &mut self.terrain,
                          &mut self.dirty);

        true
    }

    pub fn refresh<F>(&mut self, mut callback: F)
            where F: FnMut(i32, i32, &BlockChunk, &BlockChunk) {
        for (cx, cy) in self.dirty.drain() {
            let chunk = match self.terrain.get_mut(&(cx, cy)) {
                None => continue,
                Some(x) => x,
            };

            let mut new_cache = chunk.base;
            let objects_map = &mut self.objects;
            build_cache(self.data,
                        |obj_x, obj_y| {
                            objects_map.get(&(obj_x, obj_y))
                                       .map_or([].as_slice(), |o| o.objects.as_slice())
                        },
                        &mut new_cache,
                        (cx, cy));

            callback(cx, cy, &chunk.cache, &new_cache);
            chunk.cache = new_cache;
            chunk.cache_status = CacheStatus::Clean;
        }
    }
}

fn invalidate_region(bounds: Region,
                     terrain: &mut RefcountedMap<(i32, i32), ChunkTerrain>,
                     dirty: &mut HashSet<(i32, i32)>) {
    let chunk_bounds = bounds.div_round_signed(CHUNK_SIZE).with_zs(0, 1);

    for point in chunk_bounds.points() {
        match terrain.get_mut(&(point.x, point.y)) {
            Some(chunk) => {
                if chunk.cache_status == CacheStatus::Clean {
                    chunk.cache_status = CacheStatus::Dirty;
                    dirty.insert((point.x, point.y));
                }
            },
            None => {},
        }
    }
}

fn merge_objects(data: &Data,
                 blocks: &mut BlockChunk,
                 objects: &[Object],
                 offset_x: i32,
                 offset_y: i32) {
    let offset = V3::new(offset_x * CHUNK_SIZE,
                         offset_y * CHUNK_SIZE,
                         0);
    let chunk_region = Region::new(scalar(0), scalar(CHUNK_SIZE));

    for obj in objects.iter() {
        let base = offset + V3::new(obj.x as i32,
                                    obj.y as i32,
                                    obj.z as i32);
        let template = data.object_templates.template(obj.template_id);

        let obj_region = Region::new(base, base + template.size);

        for point in obj_region.intersect(&chunk_region).points() {
            let chunk_idx = chunk_region.index(&point);
            let obj_idx = obj_region.index(&point);
            blocks[chunk_idx] = template.blocks[obj_idx];
        }
    }
}

fn build_cache<'a, F>(data: &Data,
                      mut get_objects: F,
                      cache: &mut BlockChunk,
                      pos: (i32, i32))
        where F: FnMut(i32, i32) -> &'a [Object] {
    let (x, y) = pos;
    for dy in range(0, 2) {
        for dx in range(0, 2) {
            let objs = get_objects(x - dx, y - dy);
            merge_objects(data, cache, objs, -dx, -dy);
        }
    }
}
