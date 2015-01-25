use std::collections::{HashMap, HashSet};

use physics::CHUNK_BITS;
use physics::v3::{Vn, V2, V3};

use data::Data;
use input::InputBits;
use types::*;
use util::{StableIdMap, Stable};
use view::ViewState;

use self::object::{Object, ObjectRef};

macro_rules! bad {
    ($ok:expr, $msg:expr) => { bad!($ok, $msg,) };
    ($ok:expr, $msg:expr, $($extra:tt)*) => {{
        error!(concat!("broken World invariant: ", $msg), $($extra)*);
        $ok = false;
    }};
}

macro_rules! check {
    ($ok:expr, $cond:expr, $($args:tt)*) => {
        if $cond {
            bad!($ok, $($args)*);
        }
    };
}

mod object;
//mod client;
//mod entity;
mod ops;


#[derive(Copy, PartialEq, Eq, Show)]
pub enum EntityAttachment {
    World,
    Chunk,
    Client(ClientId),
}

#[derive(Copy, PartialEq, Eq, Show)]
enum StructureAttachment {
    World,
    Chunk,
}

#[derive(Copy, PartialEq, Eq, Show)]
enum InventoryAttachment {
    World,
    Client(ClientId),
    Entity(EntityId),
    Structure(StructureId),
}


pub struct Client {
    pawn: Option<EntityId>,
    current_input: InputBits,
    chunk_offset: (u8, u8),
    view_state: ViewState,

    stable_id: StableId,
    child_entities: HashSet<EntityId>,
    child_inventories: HashSet<InventoryId>,
}
impl_IntrusiveStableId!(Client, stable_id);

struct TerrainChunk {
    blocks: [BlockId; 1 << (3 * CHUNK_BITS)],
}

pub struct Motion;

pub struct Entity {
    motion: Motion,
    anim: AnimId,
    facing: V3,

    stable_id: StableId,
    attachment: EntityAttachment,
    child_inventories: HashSet<InventoryId>,
}
impl_IntrusiveStableId!(Entity, stable_id);

struct Structure {
    pos: V3,
    offset: (u8, u8, u8),
    template: TemplateId,

    stable_id: StableId,
    attachment: StructureAttachment,
    child_inventories: Vec<InventoryId>,
}
impl_IntrusiveStableId!(Structure, stable_id);

struct Inventory {
    contents: HashMap<String, u8>,

    stable_id: StableId,
    attachment: InventoryAttachment,
}
impl_IntrusiveStableId!(Inventory, stable_id);


struct World<'d> {
    data: &'d Data,
    clients: StableIdMap<ClientId, Client>,
    terrain_chunks: HashMap<V2, TerrainChunk>,
    entities: StableIdMap<EntityId, Entity>,
    structures: StableIdMap<StructureId, Structure>,
    inventories: StableIdMap<InventoryId, Inventory>,
}

pub enum Update {
    ClientViewReset(ClientId),
}

impl<'d> World<'d> {
    pub fn create_client<'a>(&'a mut self, chunk_offset: (u8, u8)) -> ObjectRef<'a, 'd, Client> {
        let client = unimplemented!(); //Client::new(chunk_offset);
        let id = self.clients.insert(client);
        ObjectRef {
            world: self,
            id: id,
        }
    }

    pub fn client(&self, id: ClientId) -> &Client {
        match self.clients.get(id) {
            None => panic!("bad ClientId: {}", id),
            Some(x) => x,
        }
    }

    pub fn client_mut<'a>(&'a mut self, id: ClientId) -> ObjectRef<'a, 'd, Client> {
        match self.clients.get_mut(id) {
            None => panic!("bad ClientId: {}", id),
            Some(_) => {},
        }

        ObjectRef {
            world: self,
            id: id,
        }
    }

    pub fn terrain_chunk(&self, id: V2) -> &TerrainChunk {
        match self.terrain_chunks.get(&id) {
            None => panic!("bad chunk id: {:?}", id),
            Some(x) => x,
        }
    }

    pub fn create_entity<'a>(&'a mut self, pos: V3, anim: AnimId) -> ObjectRef<'a, 'd, Entity> {
        let entity = unimplemented!(); //Entity::new(pos, anim);
        let id = self.entities.insert(entity);
        ObjectRef {
            world: self,
            id: id,
        }
    }

    pub fn entity(&self, id: EntityId) -> &Entity {
        match self.entities.get(id) {
            None => panic!("bad EntityId: {}", id),
            Some(x) => x,
        }
    }

    pub fn entity_mut<'a>(&'a mut self, id: EntityId) -> ObjectRef<'a, 'd, Entity> {
        match self.entities.get_mut(id) {
            None => panic!("bad EntityId: {}", id),
            Some(_) => {},
        }

        ObjectRef {
            world: self,
            id: id,
        }
    }

    pub fn structure(&self, id: StructureId) -> &Structure {
        match self.structures.get(id) {
            None => panic!("bad StructureId: {}", id),
            Some(x) => x,
        }
    }

    pub fn structure_mut<'a>(&'a mut self, id: StructureId) -> ObjectRef<'a, 'd, Structure> {
        match self.structures.get_mut(id) {
            None => panic!("bad StructureId: {}", id),
            Some(_) => {},
        }

        ObjectRef {
            world: self,
            id: id,
        }
    }

    pub fn inventory(&self, id: InventoryId) -> &Inventory {
        match self.inventories.get(id) {
            None => panic!("bad InventoryId: {}", id),
            Some(x) => x,
        }
    }

    pub fn inventory_mut<'a>(&'a mut self, id: InventoryId) -> ObjectRef<'a, 'd, Inventory> {
        match self.inventories.get_mut(id) {
            None => panic!("bad InventoryId: {}", id),
            Some(_) => {},
        }

        ObjectRef {
            world: self,
            id: id,
        }
    }

    pub fn record(&mut self, update: Update) {
        // TODO
    }
}

impl Client {
}

impl Entity {
    pub fn pos(&self, now: Time) -> V3 {
        // TODO
        V3::new(1, 2, 3)
    }
}
