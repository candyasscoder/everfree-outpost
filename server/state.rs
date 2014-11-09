use std::collections::HashMap;
use std::rand::{IsaacRng, Rng, SeedableRng};
use std::u16;

use physics;
use physics::{CHUNK_SIZE, CHUNK_BITS, CHUNK_MASK, TILE_SIZE};
use physics::v3::{V3, scalar};

use types::{Time, Duration, ClientId, EntityId, AnimId, BlockId, TileId, DURATION_MAX};

const CHUNK_TOTAL: uint = 1 << (3 * CHUNK_BITS);
type Chunk = [BlockId, ..CHUNK_TOTAL];

const LOCAL_BITS: uint = 3;
const LOCAL_SIZE: i32 = 1 << LOCAL_BITS;
const LOCAL_MASK: i32 = LOCAL_SIZE - 1;
const LOCAL_TOTAL: uint = 1 << (2 * LOCAL_BITS);

pub struct Terrain {
    pub chunks: [Chunk, ..LOCAL_TOTAL],
}

impl physics::ShapeSource for Terrain {
    fn get_shape(&self, pos: V3) -> physics::Shape {
        let V3 { x: tile_x, y: tile_y, z: tile_z } = pos & scalar(CHUNK_MASK);
        let V3 { x: chunk_x, y: chunk_y, z: _ } = (pos >> CHUNK_BITS) & scalar(LOCAL_MASK);

        let chunk_idx = chunk_y * LOCAL_SIZE + chunk_x;
        let tile_idx = (tile_z * CHUNK_SIZE + tile_y) * CHUNK_SIZE + tile_x;

        let block = self.chunks[chunk_idx as uint][tile_idx as uint];
        // TODO: don't hardcode
        if block == 0 {
            physics::Empty
        } else if block <= 4 {
            physics::Floor
        } else {
            physics::Solid
        }
    }
}



const ANIM_DIR_COUNT: AnimId = 8;

pub struct Entity {
    pub start_time: Time,
    pub duration: Duration,
    pub start_pos: V3,
    pub end_pos: V3,
    pub anim: AnimId,
}

impl Entity {
    pub fn pos(&self, now: Time) -> V3 {
        let current = now - self.start_time;

        if current < self.duration as Time {
            let offset = (self.end_pos - self.start_pos) *
                    scalar(current as i32) / scalar(self.duration as i32);
            self.start_pos + offset
        } else {
            self.end_pos
        }
    }

    pub fn update(&mut self, map: &Terrain, now: Time, input: InputBits) {
        let dx = if input.contains(INPUT_LEFT) { -1 } else { 0 } +
                 if input.contains(INPUT_RIGHT) { 1 } else { 0 };
        let dy = if input.contains(INPUT_UP) { -1 } else { 0 } +
                 if input.contains(INPUT_DOWN) { 1 } else { 0 };
        let speed =
            if dx == 0 && dy == 0 { 0 }
            else if input.contains(INPUT_RUN) { 3 }
            else { 1 };

        let idx = (3 * (dx + 1) + (dy + 1)) as uint;
        let old_anim = self.anim % ANIM_DIR_COUNT;
        let anim_dir = [5, 4, 3, 6, old_anim, 2, 7, 0, 1][idx];
        let anim = anim_dir + speed * ANIM_DIR_COUNT;

        let world_start_pos = self.pos(now);
        let world_base = base_chunk(world_start_pos);
        let local_base = offset_base_chunk(world_base, (0, 0));

        let start_pos = world_to_local(world_start_pos, world_base, local_base);
        let size = scalar(32);
        let velocity = V3::new(dx, dy, 0) * scalar(50 * speed as i32);
        let (mut end_pos, mut dur) = physics::collide(map, start_pos, size, velocity);

        if dur == 0 {
            dur = DURATION_MAX as i32;
        } else if dur > DURATION_MAX as i32 {
            let offset = end_pos - start_pos;
            end_pos = start_pos + offset * scalar(DURATION_MAX as i32) / scalar(dur);
        }

        let (end_pos, dur) = (end_pos, dur);
        let world_end_pos = local_to_world(end_pos, world_base, local_base);

        self.start_time = now;
        self.duration = dur as Duration;
        self.start_pos = world_start_pos;
        self.end_pos = world_end_pos;
        self.anim = anim;
    }

    pub fn end_time(&self) -> Time {
        self.start_time + self.duration as Time 
    }
}


bitflags! {
    flags InputBits: u16 {
        const INPUT_LEFT =      0x0001,
        const INPUT_RIGHT =     0x0002,
        const INPUT_UP =        0x0004,
        const INPUT_DOWN =      0x0008,
        const INPUT_RUN =       0x0010,
    }
}


pub struct Client {
    pub entity_id: EntityId,
    pub current_input: InputBits,
    pub chunk_offset: (u8, u8),
}


pub struct ClientEntity<'a> {
    pub client: &'a Client,
    pub entity: &'a Entity,
}


pub struct ClientEntityMut<'a> {
    pub client: &'a mut Client,
    pub entity: &'a mut Entity,
}

impl<'a> ClientEntityMut<'a> {
    pub fn imm(&self) -> ClientEntity {
        ClientEntity {
            client: self.client,
            entity: self.entity,
        }
    }
}


pub struct State {
    pub map: Terrain,
    pub entities: HashMap<EntityId, Entity>,
    pub clients: HashMap<ClientId, Client>,
}

impl State {
    pub fn new() -> State {
        State {
            map: Terrain { chunks: [[0, ..CHUNK_TOTAL], ..LOCAL_TOTAL] },
            entities: HashMap::new(),
            clients: HashMap::new(),
        }
    }

    pub fn init_terrain(&mut self) {
        let mut rng: IsaacRng = SeedableRng::from_seed([1,2,3,6].as_slice().as_slice());
        for i in range(0, LOCAL_TOTAL) {
            for j in range(0, (CHUNK_SIZE * CHUNK_SIZE) as uint) {
                if rng.gen_range(0, 10) == 0u8 {
                    self.map.chunks[i][j] = 0;
                } else {
                    self.map.chunks[i][j] = 1;
                }
            }
        }
    }

    pub fn get_terrain_rle16(&self, idx: uint) -> Vec<u16> {
        let mut result = Vec::new();

        let mut iter = self.map.chunks[idx].iter().peekable();
        while !iter.is_empty() {
            let cur = *iter.next().unwrap();

            if iter.peek().map(|x| **x) != Some(cur) {
                result.push(cur);
            } else {
                let mut count = 1u;
                while iter.peek().map(|x| **x) == Some(cur) {
                    iter.next();
                    count += 1;
                }
                result.push(0xf000 | count as u16);
                result.push(cur);
            }
        }

        result
    }

    pub fn client_entity(&self, id: ClientId) -> Option<ClientEntity> {
        let client = match self.clients.find(&id) {
            Some(c) => c,
            None => return None,
        };
        let entity = &self.entities[client.entity_id];
        Some(ClientEntity {
            client: client,
            entity: entity,
        })
    }

    pub fn client_entity_mut(&mut self, id: ClientId) -> Option<ClientEntityMut> {
        let client = match self.clients.find_mut(&id) {
            Some(c) => c,
            None => return None,
        };
        let entity = &mut self.entities[client.entity_id];
        Some(ClientEntityMut {
            client: client,
            entity: entity,
        })
    }

    pub fn add_client(&mut self, now: Time, id: ClientId) {
        let entity = Entity {
            start_time: now,
            duration: u16::MAX,
            start_pos: V3::new(100 - 16, 100 - 16, 0),
            end_pos: V3::new(100 - 16, 100 - 16, 0),
            anim: 0,
        };

        let client = Client {
            entity_id: id as EntityId,
            current_input: InputBits::empty(),
            chunk_offset: (0, 0),
        };

        self.entities.insert(id as EntityId, entity);
        self.clients.insert(id, client);
    }

    pub fn remove_client(&mut self, id: ClientId) {
        self.entities.remove(&(id as EntityId));
        self.clients.remove(&id);
    }

    pub fn update_physics(&mut self, now: Time, id: ClientId) -> bool {
        let client = &mut self.clients[id];
        let entity = &mut self.entities[client.entity_id];
        if now < entity.end_time() {
            return false;
        }
        entity.update(&self.map, now, client.current_input);
        true
    }

    pub fn update_input(&mut self, now: Time, id: ClientId, input: InputBits) -> bool {
        if !self.clients.contains_key(&id) {
            return false;
        }
        let client = &mut self.clients[id];
        if client.current_input == input {
            return false;
        }
        client.current_input = input;

        let entity = &mut self.entities[client.entity_id];
        entity.update(&self.map, now, input);

        log!(10, "client entity moves {} -> {} for dur {}",
             entity.start_pos, entity.end_pos, entity.duration);
        true
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
