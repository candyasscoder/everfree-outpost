use std::collections::HashSet;

use physics::{CHUNK_SIZE, CHUNK_BITS};
use physics::v3::{Vn, V3, Region, scalar};
use physics::Shape::{Empty, Floor};

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

impl Object {
    fn new(template_id: u32, pos: V3) -> (Object, (i32, i32)) {
        let chunk = pos.div_floor(scalar(CHUNK_SIZE));
        let offset = pos - chunk * scalar(CHUNK_SIZE);
        let obj = Object {
            template_id: template_id,
            x: offset.x as u8,
            y: offset.y as u8,
            z: offset.z as u8,
        };
        (obj, (chunk.x, chunk.y))
    }

    pub fn offset(&self) -> V3 {
        V3::new(self.x as i32,
                self.y as i32,
                self.z as i32)
    }

    pub fn pos(&self, chunk: (i32, i32)) -> V3 {
        V3::new(chunk.0, chunk.1, 0) * scalar(CHUNK_SIZE) + self.offset()
    }

    pub fn size(&self, data: &Data) -> V3 {
        data.object_templates.template(self.template_id).size
    }

    pub fn bounds(&self, data: &Data, chunk: (i32, i32)) -> Region {
        let pos = self.pos(chunk);
        let size = self.size(data);
        Region::new(pos, pos + size)
    }
}

pub type ObjectIndex = (i32, i32, usize);

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

    pub fn find_object_at_point(&self,
                                point: V3) -> Option<(ObjectIndex, &Object)> {
        let point_chunk = point.div_floor(scalar(CHUNK_SIZE));

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

                    if region.contains(point) {
                        return Some(((cx, cy, idx), obj));
                    }
                }
            }
        }

        None
    }

    pub fn create_object(&mut self,
                         pos: V3,
                         template_id: u32) -> Option<ObjectIndex> {
        let (obj, (cx, cy)) = Object::new(template_id, pos);
        let bounds = obj.bounds(self.data, (cx, cy));

        if !self.check_clear(bounds) {
            info!("can't create object at {:?}", bounds);
            return None;
        }

        let objs = match self.objects.get_mut(&(cx, cy)) {
            None => return None,
            Some(x) => x,
        };
        objs.objects.push(obj);

        invalidate_region(bounds, &mut self.terrain, &mut self.dirty);

        Some((cx, cy, objs.objects.len() - 1))
    }

    pub fn delete_object(&mut self,
                         obj_idx: ObjectIndex) -> Option<Object> {
        let (cx, cy, i) = obj_idx;

        let objs = match self.objects.get_mut(&(cx, cy)) {
            None => return None,
            Some(x) => x,
        };
        if i >= objs.objects.len() {
            return None;
        }

        let obj = objs.objects.swap_remove(i);
        let bounds = obj.bounds(self.data, (cx, cy));
        invalidate_region(bounds, &mut self.terrain, &mut self.dirty);

        Some(obj)
    }

    pub fn get_object(&self, obj_idx: ObjectIndex) -> Option<&Object> {
        let (cx, cy, i) = obj_idx;
        self.objects.get(&(cx, cy))
            .and_then(|o| o.objects.get(i))
    }

    pub fn get_object_mut(&mut self, obj_idx: ObjectIndex) -> Option<&mut Object> {
        let (cx, cy, i) = obj_idx;
        self.objects.get_mut(&(cx, cy))
            .and_then(|o| o.objects.get_mut(i))
    }

    pub fn alter_object(&mut self,
                        obj_idx: ObjectIndex,
                        new_pos: Option<V3>,
                        new_template_id: Option<u32>) -> Option<ObjectIndex> {
        let (cx, cy, i) = obj_idx;

        let old_obj = match self.delete_object(obj_idx) {
            None => return None,
            Some(obj) => obj,
        };

        let old_pos = old_obj.pos((cx, cy));
        let old_template_id = old_obj.template_id;

        let result = self.create_object(new_pos.unwrap_or(old_pos),
                                        new_template_id.unwrap_or(old_template_id));

        if result.is_none() {
            // Failed to create the object in the new position.  Instead create one back in the old
            // position, so we get back to the same state as when we started.
            self.create_object(old_pos, old_template_id)
                .expect("invariant broken: failed to replace object in its original location");
            // Make sure it has the same index in the list, too.
            let objs = &mut self.objects.get_mut(&(cx, cy)).unwrap().objects;
            let new_i = objs.len() - 1;
            objs.swap(i, new_i);
        }

        result
    }

    pub fn move_object(&mut self,
                       obj_idx: ObjectIndex,
                       new_pos: V3) -> Option<ObjectIndex> {
        self.alter_object(obj_idx, Some(new_pos), None)
    }

    pub fn replace_object(&mut self,
                          obj_idx: ObjectIndex,
                          new_template_id: u32) -> Option<ObjectIndex> {
        self.alter_object(obj_idx, None, Some(new_template_id))
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

    fn check_clear(&self, bounds: Region) -> bool {
        let chunks = bounds.div_round_signed(CHUNK_SIZE).with_zs(0, 1);

        for point in chunks.points() {
            if let Some(objs) = self.objects.get(&(point.x, point.y)) {
                let base = point * scalar(CHUNK_SIZE);
                for obj in objs.objects.iter() {
                    let pos = base + obj.offset();
                    let template = self.data.object_templates.template(obj.template_id);
                    let obj_bounds = Region::new(pos, pos + template.size);
                    if bounds.overlaps(obj_bounds) {
                        info!("check_clear: collision with object bounds {:?}",
                              obj_bounds);
                        return false;
                    }
                }
            }

            if let Some(terrain) = self.terrain.get(&(point.x, point.y)) {
                let chunk_bounds = Region::new(point, point + scalar(1)) * scalar(CHUNK_SIZE);

                for point in bounds.intersect(chunk_bounds).points() {
                    let idx = chunk_bounds.index(point);
                    match self.data.block_data.shape(terrain.base[idx]) {
                        Empty => {},
                        Floor if point.z == bounds.min.z => {},
                        s => {
                            info!("check_clear: hit {:?} terrain at {:?}",
                                  s, point);
                            return false;
                        }
                    }
                }
            }
        }

        true
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

        for point in obj_region.intersect(chunk_region).points() {
            let chunk_idx = chunk_region.index(point);
            let obj_idx = obj_region.index(point);
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
