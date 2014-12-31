use std::collections::HashMap;
use std::collections::hash_map;
use std::iter::range_inclusive;
use std::rand::{Rng, SeedableRng, XorShiftRng};
use std::u16;

use physics;
use physics::{Shape, ShapeSource};
use physics::{CHUNK_SIZE, CHUNK_BITS, CHUNK_MASK, TILE_SIZE, TILE_BITS};
use physics::v3::{V3, scalar, Region};

use types::{Time, Duration, ClientId, EntityId, AnimId, BlockId, DURATION_MAX};
use view::ViewState;
use input::{InputBits, INPUT_LEFT, INPUT_RIGHT, INPUT_UP, INPUT_DOWN, INPUT_RUN};
use input::{ActionBits, ACTION_USE};
use data::{Data, BlockData};
use gen::TerrainGenerator;

pub use self::terrain_entry::{TerrainEntry, BaseTerrainRef, ObjectsRef};
use self::StateChange::ChunkUpdate;


const CHUNK_TOTAL: uint = 1 << (3 * CHUNK_BITS);
pub type Chunk = [BlockId, ..CHUNK_TOTAL];
static EMPTY_CHUNK: Chunk = [0, ..CHUNK_TOTAL];

pub const LOCAL_BITS: uint = 3;
pub const LOCAL_SIZE: i32 = 1 << LOCAL_BITS;
pub const LOCAL_MASK: i32 = LOCAL_SIZE - 1;
pub const LOCAL_TOTAL: uint = 1 << (2 * LOCAL_BITS);

pub struct LocalTerrain<'a> {
    pub chunks: [Option<&'a Chunk>, ..LOCAL_TOTAL],
}

impl<'a> ShapeSource for (&'a LocalTerrain<'a>, &'a BlockData) {
    fn get_shape(&self, pos: V3) -> Shape {
        let &(map, block_data) = self;

        let V3 { x: tile_x, y: tile_y, z: tile_z } = pos & scalar(CHUNK_MASK);
        let V3 { x: chunk_x, y: chunk_y, z: _ } = (pos >> CHUNK_BITS) & scalar(LOCAL_MASK);

        let chunk_idx = chunk_y * LOCAL_SIZE + chunk_x;
        let tile_idx = (tile_z * CHUNK_SIZE + tile_y) * CHUNK_SIZE + tile_x;

        match map.chunks[chunk_idx as uint] {
            None => Shape::Empty,
            Some(chunk) => block_data.shape(chunk[tile_idx as uint]),
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
    pub facing: V3,
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

    pub fn update<S>(&mut self, shape_source: &S, now: Time, input: InputBits)
            where S: ShapeSource {
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
        let (mut end_pos, mut dur) = physics::collide(shape_source, start_pos, size, velocity);

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
        if dx != 0 || dy != 0 {
            self.facing = V3::new(dx, dy, 0);
        }
    }

    pub fn end_time(&self) -> Time {
        self.start_time + self.duration as Time 
    }
}


pub struct Client {
    pub entity_id: EntityId,
    pub current_input: InputBits,
    pub chunk_offset: (u8, u8),
    pub view_state: ViewState,
}


pub struct ClientEntity<'a> {
    pub client: &'a Client,
    pub entity: &'a Entity,
}

impl<'a> ClientEntity<'a> {
    fn new(client: &'a Client, entity: &'a Entity) -> ClientEntity<'a> {
        ClientEntity {
            client: client,
            entity: entity,
        }
    }
}

pub struct ClientEntityMut<'a> {
    pub client: &'a mut Client,
    pub entity: &'a mut Entity,
}


pub type Terrain<'a> = HashMap<(i32, i32), TerrainEntry<'a>>;

pub struct ObjectTemplate {
    size: V3,
    blocks: Vec<BlockId>,
}

pub struct Object {
    template_id: u32,
    x: i8,
    y: i8,
    z: i8,
}


mod terrain_entry {
    use super::{Chunk, Object};
    use physics::v3::{V3, scalar, Region};
    use physics::CHUNK_SIZE;
    use data::Data;


    pub struct TerrainEntry<'a> {
        data: &'a Data,
        ref_count: u32,
        base_terrain: Chunk,
        objects: Vec<Object>,
        block_cache: Chunk,
    }


    pub struct BaseTerrainRef<'a, 'b: 'a> {
        owner: &'a mut TerrainEntry<'b>,
    }

    impl<'a, 'b> Deref<Chunk> for BaseTerrainRef<'a, 'b> {
        fn deref(&self) -> &Chunk {
            &self.owner.base_terrain
        }
    }

    impl<'a, 'b> DerefMut<Chunk> for BaseTerrainRef<'a, 'b> {
        fn deref_mut(&mut self) -> &mut Chunk {
            &mut self.owner.base_terrain
        }
    }

    #[unsafe_destructor]
    impl<'a, 'b> Drop for BaseTerrainRef<'a, 'b> {
        fn drop(&mut self) {
            self.owner.refresh()
        }
    }


    pub struct ObjectsRef<'a, 'b: 'a> {
        owner: &'a mut TerrainEntry<'b>,
    }

    impl<'a, 'b> Deref<Vec<Object>> for ObjectsRef<'a, 'b> {
        fn deref(&self) -> &Vec<Object> {
            &self.owner.objects
        }
    }

    impl<'a, 'b> DerefMut<Vec<Object>> for ObjectsRef<'a, 'b> {
        fn deref_mut(&mut self) -> &mut Vec<Object> {
            &mut self.owner.objects
        }
    }

    #[unsafe_destructor]
    impl<'a, 'b> Drop for ObjectsRef<'a, 'b> {
        fn drop(&mut self) {
            self.owner.refresh()
        }
    }


    impl<'a> TerrainEntry<'a> {
        pub fn new(data: &'a Data, base: V3, chunk: Chunk, objs: Vec<V3>) -> TerrainEntry<'a> {
            let tree_id = data.object_templates.get_id("tree");
            let objects = objs.into_iter()
                              .map(|pos| {
                                       let pos = pos - base;
                                       Object {
                                           template_id: tree_id,
                                           x: pos.x as i8,
                                           y: pos.y as i8,
                                           z: pos.z as i8,
                                       }
                                   })
                              .collect();

            let mut entry = TerrainEntry {
                data: data,
                ref_count: 1,
                base_terrain: chunk,
                objects: objects,
                block_cache: chunk,
            };
            entry.refresh();
            entry
        }

        fn refresh(&mut self) {
            self.block_cache = self.base_terrain;

            let bounds = Region::new(scalar(0), scalar(CHUNK_SIZE));
            let block_cache = &mut self.block_cache;
            let mut set = |&mut: pos, block| {
                if !bounds.contains(&pos) {
                    return;
                }

                block_cache[bounds.index(&pos)] = block;
            };

            for obj in self.objects.iter() {
                let template = self.data.object_templates.template(obj.template_id);
                let min = V3::new(obj.x as i32,
                                  obj.y as i32,
                                  obj.z as i32) + bounds.min;
                let obj_bounds = Region::new(min, min + template.size);

                for pos in obj_bounds.points() {
                    let idx = obj_bounds.index(&pos);
                    set(pos, template.blocks[idx]);
                }
            }
        }

        pub fn retain(&mut self) {
            self.ref_count += 1;
        }

        pub fn release(&mut self) -> bool {
            self.ref_count -= 1;
            self.ref_count == 0
        }

        pub fn base_terrain(&self) -> &Chunk {
            &self.base_terrain
        }

        pub fn base_terrain_mut<'b>(&'b mut self) -> BaseTerrainRef<'a, 'b> {
            BaseTerrainRef { owner: self }
        }

        pub fn objects(&self) -> &[Object] {
            self.objects.as_slice()
        }

        pub fn objects_mut<'b>(&'b mut self) -> ObjectsRef<'a, 'b> {
            ObjectsRef { owner: self }
        }

        pub fn blocks(&self) -> &Chunk {
            &self.block_cache
        }
    }
}


#[deriving(Show)]
pub enum StateChange {
    ChunkUpdate(i32, i32),
}


pub struct State<'a> {
    pub data: &'a Data,
    pub map: Terrain<'a>,
    pub entities: HashMap<EntityId, Entity>,
    pub clients: HashMap<ClientId, Client>,
    pub terrain_gen: TerrainGenerator,
    pub rng: XorShiftRng,
}

impl<'a> State<'a> {
    pub fn new(data: &'a Data) -> State<'a> {
        State {
            data: data,
            map: HashMap::new(),
            entities: HashMap::new(),
            clients: HashMap::new(),
            terrain_gen: TerrainGenerator::new(12345),
            rng: SeedableRng::from_seed([12345, 45205314, 65412562, 940534205]),
        }
    }

    pub fn get_terrain_rle16(&self, cx: i32, cy: i32) -> Vec<u16> {
        let mut result = Vec::new();

        let chunk = self.map.get(&(cx, cy)).map_or(&EMPTY_CHUNK, |c| c.blocks());

        let mut iter = chunk.iter().peekable();
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

    pub fn client_ids(&self) -> Vec<ClientId> {
        self.clients.keys().map(|&x| x).collect()
    }

    pub fn client_entity(&self, id: ClientId) -> Option<ClientEntity> {
        let client = match self.clients.get(&id) {
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
        let client = match self.clients.get_mut(&id) {
            Some(c) => c,
            None => return None,
        };
        let entity = &mut self.entities[client.entity_id];
        Some(ClientEntityMut {
            client: client,
            entity: entity,
        })
    }

    pub fn client_entities(&self) -> ClientEntities {
        ClientEntities {
            clients: self.clients.iter(),
            entities: &self.entities,
        }
    }

    pub fn add_client(&mut self, now: Time, id: ClientId) {
        let pos = V3::new(250, 250, 0);
        let offset = V3::new(16, 16, 0);

        let entity = Entity {
            start_time: now,
            duration: u16::MAX,
            start_pos: pos - offset,
            end_pos: pos - offset,
            anim: 0,
            facing: V3::new(1, 0, 0),
        };

        let chunk_offset = (self.rng.gen_range(0, 8),
                            self.rng.gen_range(0, 8));

        let client = Client {
            entity_id: id as EntityId,
            current_input: InputBits::empty(),
            chunk_offset: chunk_offset,
            view_state: ViewState::new(pos),
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
        let terrain = build_local_terrain(&self.map, now, ClientEntity::new(client, entity), false);
        entity.update(&(&terrain, &self.data.block_data), now, client.current_input);
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
        let terrain = build_local_terrain(&self.map, now, ClientEntity::new(client, entity), false);
        entity.update(&(&terrain, &self.data.block_data), now, input);

        true
    }

    pub fn perform_action(&mut self, now: Time, id: ClientId, action: ActionBits) -> Vec<StateChange> {
        let pos_px = {
            let ce = match self.client_entity(id) {
                Some(ce) => ce,
                None => return Vec::new(),
            };
            ce.entity.pos(now) + scalar(16) + scalar(32) * ce.entity.facing
        };
        let pos = pos_px.div_floor(&scalar(TILE_SIZE));
        let chunk = pos.div_floor(&scalar(CHUNK_SIZE));

        log!(10, "perform action: px={}; tile={}; chunk={}", pos_px, pos, chunk);

        let tree_id = self.data.object_templates.get_id("tree");
        let stump_id = self.data.object_templates.get_id("stump");

        let found_idx = {
            let objects = match self.map.get(&(chunk.x, chunk.y)) {
                Some(c) => c.objects(),
                None => return Vec::new(),
            };

            log!(10, "got {} objects for chunk", objects.len());

            let mut found_idx = None;
            for (i, obj) in objects.iter().enumerate() {
                if obj.template_id != tree_id {
                    continue;
                }

                let obj_base = V3::new(obj.x as i32,
                                       obj.y as i32,
                                       obj.z as i32);
                let offset = pos - obj_base;
                if Region::around(V3::new(2, 1, 0), 1).contains(&offset) {
                    log!(10, "found hit on tree {} at offset {}", i, offset);
                    found_idx = Some(i);
                    break;
                }
            }

            found_idx
        };

        log!(10, "hit index: {}", found_idx);

        let mut updates = Vec::new();

        match found_idx {
            Some(idx) => {
                self.map[(chunk.x, chunk.y)]
                    .objects_mut()[idx].template_id = stump_id;
                updates.push(ChunkUpdate(chunk.x, chunk.y));
            },
            None => {},
        }

        updates
    }

    pub fn load_chunk(&mut self, cx: i32, cy: i32) {
        use std::collections::hash_map::Entry::{Vacant, Occupied};
        match self.map.entry((cx, cy)) {
            Vacant(e) => {
                log!(10, "LOAD {} {} -> 1 (INIT)", cx, cy);
                let (chunk, objs) = self.terrain_gen.generate_chunk(&self.data.block_data, cx, cy);
                e.set(TerrainEntry::new(self.data,
                                        V3::new(cx, cy, 0) * scalar(CHUNK_SIZE),
                                        chunk, objs));
            },
            Occupied(mut e) => {
                e.get_mut().retain();
                log!(10, "LOAD {} {}", cx, cy);
            },
        }
    }

    pub fn unload_chunk(&mut self, cx: i32, cy: i32) {
        use std::collections::hash_map::Entry::{Vacant, Occupied};
        match self.map.entry((cx, cy)) {
            Vacant(_) => return,
            Occupied(mut e) => {
                if e.get_mut().release() {
                    log!(10, "UNLOAD {} {} -> 0 (DEAD)", cx, cy);
                    e.take();
                } else {
                    log!(10, "UNLOAD {} {}", cx, cy);
                }
            },
        }
    }
}


pub struct ClientEntities<'a> {
    clients: hash_map::Entries<'a, ClientId, Client>,
    entities: &'a HashMap<EntityId, Entity>,
}

impl<'a> Iterator<(ClientId, ClientEntity<'a>)> for ClientEntities<'a> {
    fn next(&mut self) -> Option<(ClientId, ClientEntity<'a>)> {
        // The loop keeps going until we have a definitive result - either we found a Client and
        // its Entity, or there are no more clients (that have corresponding entities).
        loop {
            let (&id, client) = match self.clients.next() {
                Some(x) => x,
                None => return None,
            };

            match self.entities.get(&client.entity_id) {
                Some(entity) => {
                    // Success - got a Client and its Entity.
                    return Some((id, ClientEntity::new(client, entity)));
                },
                None => {
                    // Failure - go around the loop again to try the next client.
                },
            };
        }
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

pub fn build_local_terrain<'a>(map: &'a Terrain,
                               now: Time,
                               ce: ClientEntity,
                               use_chunk_offset: bool) -> LocalTerrain<'a> {
    let mut local = LocalTerrain {
        chunks: [None, ..LOCAL_TOTAL],
    };

    let pos = ce.entity.pos(now) >> (TILE_BITS + CHUNK_BITS);
    let offset = if use_chunk_offset { ce.client.chunk_offset } else { (0, 0) };

    for y in range_inclusive(-2, 3) {
        for x in range_inclusive(-2, 2) {
            let cx = pos.x + x;
            let cy = pos.y + y;

            let lx = (cx + offset.0 as i32) & LOCAL_MASK;
            let ly = (cy + offset.1 as i32) & LOCAL_MASK;
            let local_idx = ly * LOCAL_SIZE + lx;
            local.chunks[local_idx as uint] = map.get(&(cx, cy)).map(|x| x.blocks());
        }
    }

    local
}
