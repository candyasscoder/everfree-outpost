use std::iter::range_inclusive;
use std::ops::{Deref, DerefMut};
use std::rand::{self, Rng, SeedableRng, XorShiftRng};

use physics;
use physics::{Shape, ShapeSource};
use physics::{CHUNK_SIZE, CHUNK_BITS, CHUNK_MASK, TILE_SIZE, TILE_BITS};
use physics::v3::{V3, V2, Vn, scalar, Region};

use types::*;
use input::{InputBits, INPUT_LEFT, INPUT_RIGHT, INPUT_UP, INPUT_DOWN, INPUT_RUN};
use input::ActionId;
use data::{Data, BlockData};
use gen::TerrainGenerator;
use script::ScriptEngine;
use terrain2;
use world;
use world::object::*;
use util::StrError;
use storage::Storage;
use world::save::{self, ObjectReader, ObjectWriter};
use script::{ReadHooks, WriteHooks};
use util::Cursor;

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

pub fn input_to_target_velocity(input: InputBits) -> V3 {
    let dx = if input.contains(INPUT_LEFT) { -1 } else { 0 } +
             if input.contains(INPUT_RIGHT) { 1 } else { 0 };
    let dy = if input.contains(INPUT_UP) { -1 } else { 0 } +
             if input.contains(INPUT_DOWN) { 1 } else { 0 };
    let speed =
        if dx == 0 && dy == 0 { 0 }
        else if input.contains(INPUT_RUN) { 3 }
        else { 1 };

    V3::new(dx, dy, 0) * scalar(50 * speed)
}

pub fn run_physics<S>(entity: &world::Entity,
                      shape_source: &S,
                      now: Time) -> (world::Motion, AnimId, V3)
        where S: ShapeSource {
    let velocity = entity.target_velocity();
    let dir = velocity.signum();
    let speed = velocity.abs().max() / 50;

    let idx = (3 * (dir.x + 1) + (dir.y + 1)) as usize;
    let old_anim = entity.anim() % ANIM_DIR_COUNT;
    let anim_dir = [5, 4, 3, 6, old_anim, 2, 7, 0, 1][idx];
    let anim = anim_dir + speed as AnimId * ANIM_DIR_COUNT;

    let world_start_pos = entity.pos(now);
    let world_base = base_chunk(world_start_pos);
    let local_base = offset_base_chunk(world_base, (0, 0));

    let start_pos = world_to_local(world_start_pos, world_base, local_base);
    // TODO: hardcoded constant based on entity size
    let size = scalar(32);
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
        if dir != scalar(0) {
            dir
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
    pub storage: Storage,
    pub script: ScriptEngine,
    pub mw: terrain2::ManagedWorld<'a>,
    pub terrain_gen: TerrainGenerator,
    pub rng: XorShiftRng,
}

impl<'a> State<'a> {
    pub fn new(data: &'a Data, storage: Storage) -> State<'a> {
        let script_dir = storage.script_dir();
        State {
            data: data,
            storage: storage,
            script: ScriptEngine::new(&script_dir),

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

    pub fn load_client<'b>(&'b mut self,
                           name: &str)
                           -> save::Result<world::object::ObjectRefMut<'b, 'a, world::Client>> {
        let chunk_offset = (self.rng.gen_range(0, 8),
                            self.rng.gen_range(0, 8));

        if let Some(file) = self.storage.open_client_file(name) {
            info!("loading client {} from file", name);
            let mut sr = ObjectReader::new(file, ReadHooks::new(&mut self.script));
            let cid = try!(sr.load_client(self.mw.world_mut()));
            let mut c = self.mw.world_mut().client_mut(cid);

            // Fix up transient bits of state that shouldn't be preserved across save/load.
            c.set_current_input(InputBits::empty());
            c.set_chunk_offset(chunk_offset);

            Ok(c)
        } else {
            info!("initializing new client {}", name);

            let pos = V3::new(64, 64, 0);
            // TODO: hardcoded constant based on entity size
            let offset = V3::new(16, 16, 0);

            let appearance = appearance_from_name(name);
            let pawn_id = self.world_mut().create_entity(pos - offset, 0, appearance)
                              .unwrap().id();

            let mut client = self.world_mut().create_client(name, chunk_offset).unwrap();
            client.set_pawn(Some(pawn_id)).unwrap();
            Ok(client)
        }
    }

    pub fn unload_client(&mut self, cid: ClientId) -> save::Result<()> {
        {
            let c = self.mw.world().client(cid);
            let file = self.storage.create_client_file(c.name());
            let mut sw = ObjectWriter::new(file, WriteHooks::new(&mut self.script));
            try!(sw.save_client(&c));
        }

        try!(self.world_mut().destroy_client(cid));
        Ok(())
    }

    pub fn update_physics(&mut self, now: Time, eid: EntityId, force: bool) -> Result<(), StrError> {
        let (motion, anim, facing) = {
            let entity = unwrap!(self.world().get_entity(eid));
            // TODO: weird bug: it should be harmless to always act like `force` is enabled, but
            // doing so causes the entity to get stuck instead of sliding along walls
            if !force && now < entity.motion().end_time() {
                return Ok(());
            }

            let terrain = build_local_terrain(&self.mw, entity.pos(now), (0, 0));
            run_physics(&*entity,
                        &(&terrain, &self.data.block_data),
                        now)
        };
        let mut e = self.world_mut().entity_mut(eid);
        e.set_motion(motion);
        e.set_anim(anim);
        e.set_facing(facing);
        Ok(())
    }

    pub fn update_input(&mut self, now: Time, id: ClientId, input: InputBits) -> Result<(), StrError> {
        let eid = {
            let mut client = unwrap!(self.world_mut().get_client_mut(id));
            if client.current_input() == input {
                return Ok(());
            }
            client.set_current_input(input);

            let mut entity = unwrap!(client.pawn_mut());
            entity.set_target_velocity(input_to_target_velocity(input));
            entity.id()
        };

        self.update_physics(now, eid, true)
    }

    pub fn perform_action(&mut self,
                          now: Time,
                          id: ClientId,
                          action: ActionId,
                          arg: u32) -> Result<(), String> {
        self.script.callback_action(self.mw.world_mut(), now, id, action, arg)
    }

    // TODO: gross type signature.  See comment in terrain2 for a possible fix.
    pub fn process_journal<P, S, F>(self_: Cursor<State<'a>, P, S>, mut f: F)
            where P: DerefMut,
                  S: Fn(&mut <P as Deref>::Target) -> &mut State<'a>,
                  F: FnMut(&mut Cursor<State<'a>, P, S>, world::Update) {
        terrain2::ManagedWorld::process_journal(self_.extend(|s| &mut s.mw), |mw, u| {
            let mut s = mw.up();
            match u {
                world::Update::ClientDestroyed(id) =>
                    s.script.callback_client_destroyed(id),
                world::Update::EntityDestroyed(id) =>
                    s.script.callback_entity_destroyed(id),
                world::Update::StructureDestroyed(id) =>
                    s.script.callback_structure_destroyed(id),
                world::Update::InventoryDestroyed(id) =>
                    s.script.callback_inventory_destroyed(id),
                _ => {},
            }
            f(&mut *s, u);
        });
    }

    pub fn load_chunk(&mut self, cx: i32, cy: i32) {
        let storage = &self.storage;
        let script = &mut self.script;
        let gen = &mut self.terrain_gen;
        let block_data = &self.data.block_data;
        let template_data = &self.data.object_templates;

        let mut rng: XorShiftRng = SeedableRng::from_seed([cx as u32, cy as u32, 10, 20]);

        self.mw.retain(V2::new(cx, cy), |w, pos| {
            if let Some(file) = storage.open_terrain_chunk_file(pos) {
                let mut sr = ObjectReader::new(file, ReadHooks::new(script));
                sr.load_terrain_chunk(w).unwrap();
            } else {
                let (blocks, points) = gen.generate_chunk(block_data, pos.x, pos.y);
                w.create_terrain_chunk(pos, Box::new(blocks)).unwrap();

                let base = pos.extend(0) * scalar(CHUNK_SIZE);
                let bounds = Region::new(base, base + scalar(CHUNK_SIZE));
                let template_ids = [
                    template_data.get_id("tree"),
                    template_data.get_id("rock"),
                ];
                for pos in points.into_iter() .filter(|&p| bounds.contains(p)) {
                    let template_id = *rng.choose(&template_ids).unwrap();
                    // TODO: need a special entry point to create a structure bypassing overlap
                    // checks
                    let mut s = match w.create_structure(pos, template_id) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    s.set_attachment(world::StructureAttachment::Chunk).unwrap();
                }

                if cx == 0 && cy == 0 {
                    let template_id = template_data.get_id("anvil");
                    let mut s = w.create_structure(scalar(0), template_id).unwrap();
                    s.set_attachment(world::StructureAttachment::Chunk).unwrap();
                }
            }
        });
    }

    pub fn unload_chunk(&mut self, cx: i32, cy: i32) {
        let storage = &self.storage;
        let script = &mut self.script;
        self.mw.release(V2::new(cx, cy), |w, pos| {
            {
                let t = w.terrain_chunk(pos);
                let file = storage.create_terrain_chunk_file(pos);
                let mut sw = ObjectWriter::new(file, WriteHooks::new(script));
                sw.save_terrain_chunk(&t).unwrap();
            }

            w.destroy_terrain_chunk(pos).unwrap();
        });
    }

    pub fn load_world(&mut self) {
        if let Some(file) = self.storage.open_world_file() {
            let mut sr = ObjectReader::new(file, ReadHooks::new(&mut self.script));
            sr.load_world(self.mw.world_mut()).unwrap();
        }
    }

    pub fn save_all(&mut self) {
        for c in self.mw.world().clients() {
            let file = self.storage.create_client_file(c.name());
            let mut sw = ObjectWriter::new(file, WriteHooks::new(&mut self.script));
            warn_on_err!(sw.save_client(&c));
        }

        for t in self.mw.world().terrain_chunks() {
            let file = self.storage.create_terrain_chunk_file(t.id());
            let mut sw = ObjectWriter::new(file, WriteHooks::new(&mut self.script));
            warn_on_err!(sw.save_terrain_chunk(&t));
        }

        let file = self.storage.create_world_file();
        let mut sw = ObjectWriter::new(file, WriteHooks::new(&mut self.script));
        warn_on_err!(sw.save_world(self.mw.world()));
    }
}

#[unsafe_destructor]
impl<'a> Drop for State<'a> {
    fn drop(&mut self) {
        self.save_all();
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

fn appearance_from_name(name: &str) -> u32 {
    let mut rng = rand::thread_rng();

    let (tribe, r, g, b) = if name.starts_with("Anon") && name.len() >= 8 {
        let mut get = |&mut: c: char, options: &str| {
            if c == options.char_at(0) {
                0
            } else if c == options.char_at(1) {
                1
            } else if c == options.char_at(2) {
                2
            } else {
                rng.gen_range(0, 3)
            }
        };

        (get(name.char_at(4), "EPU"),
         get(name.char_at(5), "123"),
         get(name.char_at(6), "123"),
         get(name.char_at(7), "123"))
    } else {
        (rng.gen_range(0, 3),
         rng.gen_range(0, 3),
         rng.gen_range(0, 3),
         rng.gen_range(0, 3))
    };

    (tribe << 6) |
    (r << 4) |
    (g << 2) |
    (b)
}
