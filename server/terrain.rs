
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
}

impl<'d> Terrain<'d> {
    pub fn new(data: &'d Data) -> Terrain<'d> {
        Terrain {
            data: data,
            terrain: RefcountedMap::new(),
            objects: RefcountedMap::new(),
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

            for dy in range(0, 2) {
                for dx in range(0, 2) {
                    let obj_x = x - dx;
                    let obj_y = y - dy;
                    let (objs, _) = objects_map.retain((obj_x, obj_y), || {
                        ChunkObjects {
                            objects: load_objects(obj_x, obj_y),
                        }
                    });

                    merge_objects(data, &mut cache, objs.objects.as_slice(), -dx, -dy);
                }
            }

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
