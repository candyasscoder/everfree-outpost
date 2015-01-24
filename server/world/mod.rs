use std::collections::HashMap;

use physics::CHUNK_BITS;
use physics::v3::{Vn, V2, V3};

use data::Data;
use types::*;
use util::{StableIdMap, Stable};

use self::object::{Object, ObjectRef};
use self::client::Client;
use self::entity::Entity;

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
mod client;
mod entity;

enum StructureAttachment {
    World,
    Chunk,
}

enum InventoryAttachment {
    World,
    Client(ClientId),
    Entity(EntityId),
    Structure(StructureId),
}



struct TerrainChunk {
    blocks: [BlockId; 1 << (3 * CHUNK_BITS)],
}


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


impl Object for Client {
    type Id = ClientId;

    fn get<'a>(world: &'a World, id: ClientId) -> Option<&'a Client> {
        world.clients.get(id)
    }

    fn get_mut<'a>(world: &'a mut World, id: ClientId) -> Option<&'a mut Client> {
        world.clients.get_mut(id)
    }
}

impl Object for Entity {
    type Id = EntityId;

    fn get<'a>(world: &'a World, id: EntityId) -> Option<&'a Entity> {
        world.entities.get(id)
    }

    fn get_mut<'a>(world: &'a mut World, id: EntityId) -> Option<&'a mut Entity> {
        world.entities.get_mut(id)
    }
}

impl Object for Structure {
    type Id = StructureId;

    fn get<'a>(world: &'a World, id: StructureId) -> Option<&'a Structure> {
        world.structures.get(id)
    }

    fn get_mut<'a>(world: &'a mut World, id: StructureId) -> Option<&'a mut Structure> {
        world.structures.get_mut(id)
    }
}

impl Object for Inventory {
    type Id = InventoryId;

    fn get<'a>(world: &'a World, id: InventoryId) -> Option<&'a Inventory> {
        world.inventories.get(id)
    }

    fn get_mut<'a>(world: &'a mut World, id: InventoryId) -> Option<&'a mut Inventory> {
        world.inventories.get_mut(id)
    }
}


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
        let client = Client::new(chunk_offset);
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
        let entity = Entity::new(pos, anim);
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
