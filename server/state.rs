use std::collections::HashMap;
use std::collections::hash_map;
use std::iter::range_inclusive;
use std::rand::{Rng, SeedableRng, XorShiftRng};
use std::u16;

use physics;
use physics::{Shape, ShapeSource};
use physics::{CHUNK_SIZE, CHUNK_BITS, CHUNK_MASK, TILE_SIZE, TILE_BITS};
use physics::v3::{V3, V2, Vn, scalar, Region};

use types::*;
use view::ViewState;
use input::{InputBits, INPUT_LEFT, INPUT_RIGHT, INPUT_UP, INPUT_DOWN, INPUT_RUN};
use input::ActionBits;
use data::{Data, BlockData};
use gen::TerrainGenerator;
use script::ScriptEngine;
use terrain2;
use world;
use world::object::{ObjectRefBase, ClientRef, ClientRefMut, EntityRefMut};
use util::StrError;

use self::StateChange::ChunkUpdate;


pub const LOCAL_BITS: usize = 3;
pub const LOCAL_SIZE: i32 = 1 << LOCAL_BITS;
pub const LOCAL_MASK: i32 = LOCAL_SIZE - 1;
pub const LOCAL_TOTAL: usize = 1 << (2 * LOCAL_BITS);

pub struct LocalTerrain<'a> {
    pub chunks: [&'a BlockChunk; LOCAL_TOTAL],
}

impl<'a> ShapeSource for (&'a LocalTerrain<'a>, &'a BlockData) {
    fn get_shape(&self, pos: V3) -> Shape {
        let &(map, block_data) = self;

        let V3 { x: tile_x, y: tile_y, z: tile_z } = pos & scalar(CHUNK_MASK);
        let V3 { x: chunk_x, y: chunk_y, z: _ } = (pos >> CHUNK_BITS) & scalar(LOCAL_MASK);

        let chunk_idx = chunk_y * LOCAL_SIZE + chunk_x;
        let tile_idx = (tile_z * CHUNK_SIZE + tile_y) * CHUNK_SIZE + tile_x;

        block_data.shape(map.chunks[chunk_idx as usize][tile_idx as usize])
    }
}



const ANIM_DIR_COUNT: AnimId = 8;

pub fn run_physics<S>(entity: &world::Entity,
                      shape_source: &S,
                      now: Time,
                      input: InputBits) -> (world::Motion, AnimId, V3)
        where S: ShapeSource {
    let dx = if input.contains(INPUT_LEFT) { -1 } else { 0 } +
             if input.contains(INPUT_RIGHT) { 1 } else { 0 };
    let dy = if input.contains(INPUT_UP) { -1 } else { 0 } +
             if input.contains(INPUT_DOWN) { 1 } else { 0 };
    let speed =
        if dx == 0 && dy == 0 { 0 }
        else if input.contains(INPUT_RUN) { 3 }
        else { 1 };

    let idx = (3 * (dx + 1) + (dy + 1)) as usize;
    let old_anim = entity.anim() % ANIM_DIR_COUNT;
    let anim_dir = [5, 4, 3, 6, old_anim, 2, 7, 0, 1][idx];
    let anim = anim_dir + speed * ANIM_DIR_COUNT;

    let world_start_pos = entity.pos(now);
    let world_base = base_chunk(world_start_pos);
    let local_base = offset_base_chunk(world_base, (0, 0));

    let start_pos = world_to_local(world_start_pos, world_base, local_base);
    let size = scalar(32);
    let velocity = V3::new(dx, dy, 0) * scalar(50 * speed as i32);
    let (mut end_pos, mut dur) = physics::collide(shape_source, start_pos, size, velocity);

    if dur == 0 {
        dur = DURATION_MAX as i32;
    } else if dur > DURATION_MAX as i32 {
        let offset = end_pos - start_pos;
        end_pos = start_pos + offset * scalar(DURATION_MAX as i32) / scalar(dur);
    }

    let (end_pos, dur) = (end_pos, dur);
    let world_end_pos = local_to_world(end_pos, world_base, local_base);

    let new_motion = world::Motion {
        start_time: now,
        duration: dur as Duration,
        start_pos: world_start_pos,
        end_pos: world_end_pos,
    };
    let new_facing = 
        if dx != 0 || dy != 0 {
            V3::new(dx, dy, 0)
        } else {
            entity.facing()
        };

    (new_motion, anim, new_facing)
}



#[derive(Show)]
pub enum StateChange {
    ChunkUpdate(i32, i32),
}


pub struct State<'a> {
    pub data: &'a Data,
    pub script: ScriptEngine,
    pub mw: terrain2::ManagedWorld<'a>,
    pub terrain_gen: TerrainGenerator,
    pub rng: XorShiftRng,
}

impl<'a> State<'a> {
    pub fn new(data: &'a Data, script_path: &str) -> State<'a> {
        State {
            data: data,
            script: ScriptEngine::new(&Path::new(script_path)),

            mw: terrain2::ManagedWorld::new(data),

            terrain_gen: TerrainGenerator::new(12345),
            rng: SeedableRng::from_seed([12345, 45205314, 65412562, 940534205]),
        }
    }

    pub fn get_terrain_rle16(&self, cx: i32, cy: i32) -> Vec<u16> {
        let mut result = Vec::new();

        let chunk = self.mw.get_terrain(V2::new(cx, cy)).unwrap();

        let mut iter = chunk.iter().peekable();
        while !iter.is_empty() {
            let cur = *iter.next().unwrap();

            if iter.peek().map(|x| **x) != Some(cur) {
                result.push(cur);
            } else {
                // TODO: check that count doesn't overflow 12 bits.
                let mut count = 1u16;
                while iter.peek().map(|x| **x) == Some(cur) {
                    iter.next();
                    count += 1;
                }
                result.push(0xf000 | count);
                result.push(cur);
            }
        }

        result
    }

    pub fn world(&self) -> &world::World<'a> {
        self.mw.world()
    }

    pub fn world_mut(&mut self) -> &mut world::World<'a> {
        self.mw.world_mut()
    }

    pub fn add_client<'b>(&'b mut self,
                          now: Time,
                          wire_id: WireId) -> world::object::ObjectRefMut<'b, 'a, world::Client> {
        let pos = V3::new(250, 250, 0);
        let offset = V3::new(16, 16, 0);

        let pawn_id = self.world_mut().create_entity(pos - offset, 0).unwrap().id();

        let chunk_offset = (self.rng.gen_range(0, 8),
                            self.rng.gen_range(0, 8));

        let mut client = self.world_mut().create_client(wire_id, chunk_offset).unwrap();
        client.set_pawn(now, Some(pawn_id)).unwrap();
        client
    }

    pub fn remove_client(&mut self, id: ClientId) {
        self.world_mut().destroy_client(id).unwrap();
    }

    pub fn update_physics(&mut self, now: Time, id: ClientId) -> Result<bool, StrError> {
        let (eid, (motion, anim, facing)) = {
            let client = unwrap!(self.world().get_client(id));
            let entity = unwrap!(client.pawn());
            if now < entity.motion().end_time() {
                return Ok(false);
            }

            let terrain = build_local_terrain(&self.mw, entity.pos(now), (0, 0));
            let phys_result = run_physics(&*entity,
                                          &(&terrain, &self.data.block_data),
                                          now,
                                          client.current_input());

            (entity.id(), phys_result)
        };
        let mut e = self.world_mut().entity_mut(eid);
        e.set_motion(motion);
        e.set_anim(anim);
        e.set_facing(facing);
        Ok(true)
    }

    pub fn update_input(&mut self, now: Time, id: ClientId, input: InputBits) -> Result<bool, StrError> {
        let (eid, (motion, anim, facing)) = {
            let client = unwrap!(self.world().get_client(id));
            if client.current_input() == input {
                return Ok(false);
            }

            let entity = unwrap!(client.pawn());

            let terrain = build_local_terrain(&self.mw, entity.pos(now), (0, 0));
            let phys_result = run_physics(&*entity,
                                          &(&terrain, &self.data.block_data),
                                          now,
                                          input);

            (entity.id(), phys_result)
        };

        self.world_mut().client_mut(id).set_current_input(input);

        let mut e = self.world_mut().entity_mut(eid);
        e.set_motion(motion);
        e.set_anim(anim);
        e.set_facing(facing);
        Ok(true)
    }

    pub fn perform_action(&mut self, now: Time, id: ClientId, action: ActionBits) -> Vec<StateChange> {
        self.script.test_callback(self.mw.world_mut(), now, id, action);

        let mut updates = Vec::new();
        let journal = self.world_mut().take_journal();
        for update in journal.into_iter() {
            match update {
                world::Update::ChunkInvalidate(pos) => {
                    self.mw.refresh_chunk(pos);
                    updates.push(ChunkUpdate(pos.x, pos.y));
                },
                _ => {},
            }
        }

        updates
    }

    pub fn load_chunk(&mut self, cx: i32, cy: i32) {
        let gen = &mut self.terrain_gen;
        let block_data = &self.data.block_data;
        let template_data = &self.data.object_templates;
        self.mw.retain(V2::new(cx, cy),
            |c| { gen.generate_chunk(block_data, c.x, c.y).0 },
            |c| {
                let base = V3::new(c.x * CHUNK_SIZE,
                                   c.y * CHUNK_SIZE,
                                   0);
                let bounds = Region::new(base, base + scalar(CHUNK_SIZE));
                let points = gen.generate_chunk(block_data, c.x, c.y).1;
                let id = template_data.get_id("tree");
                points.into_iter()
                      .filter(|&p| bounds.contains(p))
                      .map(|p| (p, id))
                      .collect()
            });
    }

    pub fn unload_chunk(&mut self, cx: i32, cy: i32) {
        self.mw.release(V2::new(cx, cy));
    }
}


pub fn base_chunk(pos: V3) -> V3 {
    let size = CHUNK_SIZE * TILE_SIZE;
    let chunk_x = (pos.x + if pos.x < 0 { -size + 1 } else { 0 }) / size;
    let chunk_y = (pos.y + if pos.y < 0 { -size + 1 } else { 0 }) / size;

    let offset = LOCAL_SIZE / 2;

    V3::new(chunk_x - offset, chunk_y - offset, 0)
}

pub fn offset_base_chunk(base_chunk: V3, base_offset: (u8, u8)) -> V3 {
    let base_off = V3::new(base_offset.0 as i32,
                           base_offset.1 as i32,
                           0);
    (base_chunk + base_off) & scalar(LOCAL_MASK)
}

pub fn local_to_world(local: V3, world_base_chunk: V3, local_base_chunk: V3) -> V3 {
    let world_base = world_base_chunk * scalar(CHUNK_SIZE * TILE_SIZE);
    let local_base = local_base_chunk * scalar(CHUNK_SIZE * TILE_SIZE);

    let offset = (local - local_base) & scalar(CHUNK_SIZE * TILE_SIZE * LOCAL_SIZE - 1);
    world_base + offset
}

pub fn world_to_local(world: V3, world_base_chunk: V3, local_base_chunk: V3) -> V3 {
    let world_base = world_base_chunk * scalar(CHUNK_SIZE * TILE_SIZE);
    let local_base = local_base_chunk * scalar(CHUNK_SIZE * TILE_SIZE);

    let offset = world - world_base;
    local_base + offset
}

pub fn build_local_terrain<'a>(mw: &'a terrain2::ManagedWorld,
                               reference_pos: V3,
                               chunk_offset: (u8, u8)) -> LocalTerrain<'a> {
    let mut local = LocalTerrain {
        chunks: [&EMPTY_CHUNK; LOCAL_TOTAL],
    };

    let pos = reference_pos >> (TILE_BITS + CHUNK_BITS);

    for y in range_inclusive(-2, 3) {
        for x in range_inclusive(-2, 2) {
            let cx = pos.x + x;
            let cy = pos.y + y;

            let lx = (cx + chunk_offset.0 as i32) & LOCAL_MASK;
            let ly = (cy + chunk_offset.1 as i32) & LOCAL_MASK;
            let local_idx = ly * LOCAL_SIZE + lx;
            local.chunks[local_idx as usize] = mw.get_terrain(V2::new(cx, cy))
                                                 .unwrap_or(&EMPTY_CHUNK);
        }
    }

    local
}
