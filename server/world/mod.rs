use std::collections::{HashMap, HashSet, hash_set};
use std::mem::replace;

use physics::CHUNK_BITS;
use physics::v3::{Vn, V2, V3, scalar};

use data::Data;
use input::InputBits;
use types::*;
use util::{StableIdMap, Stable, StableIdMapIter};
use view::ViewState;

use self::object::{Object, ObjectRef, ObjectRefMut};
pub use self::ops::OpResult;

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

pub mod object;
mod ops;
mod debug;


#[derive(Copy, PartialEq, Eq, Show)]
pub enum EntityAttachment {
    World,
    Chunk,
    Client(ClientId),
}

#[derive(Copy, PartialEq, Eq, Show)]
pub enum StructureAttachment {
    World,
    Chunk,
}

#[derive(Copy, PartialEq, Eq, Show)]
pub enum InventoryAttachment {
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

pub struct TerrainChunk {
    blocks: [BlockId; 1 << (3 * CHUNK_BITS)],
}

pub struct Entity {
    motion: Motion,
    anim: AnimId,
    facing: V3,

    stable_id: StableId,
    attachment: EntityAttachment,
    child_inventories: HashSet<InventoryId>,
}
impl_IntrusiveStableId!(Entity, stable_id);

pub struct Structure {
    pos: V3,
    offset: (u8, u8, u8),
    template: TemplateId,

    stable_id: StableId,
    attachment: StructureAttachment,
    child_inventories: HashSet<InventoryId>,
}
impl_IntrusiveStableId!(Structure, stable_id);

pub struct Inventory {
    contents: HashMap<String, u8>,

    stable_id: StableId,
    attachment: InventoryAttachment,
}
impl_IntrusiveStableId!(Inventory, stable_id);


pub struct World<'d> {
    data: &'d Data,
    journal: Vec<Update>,

    clients: StableIdMap<ClientId, Client>,
    terrain_chunks: HashMap<V2, TerrainChunk>,
    entities: StableIdMap<EntityId, Entity>,
    structures: StableIdMap<StructureId, Structure>,
    inventories: StableIdMap<InventoryId, Inventory>,

    structures_by_chunk: HashMap<V2, HashSet<StructureId>>,
}

pub enum Update {
    ClientViewReset(ClientId),
    ChunkInvalidate(V2),
    EntityMotionChange(EntityId),
}

impl<'d> World<'d> {
    pub fn new(data: &'d Data) -> World<'d> {
        World {
            data: data,
            journal: Vec::new(),

            clients: StableIdMap::new(),
            terrain_chunks: HashMap::new(),
            entities: StableIdMap::new(),
            structures: StableIdMap::new(),
            inventories: StableIdMap::new(),

            structures_by_chunk: HashMap::new(),
        }
    }

    pub fn record(&mut self, update: Update) {
        self.journal.push(update);
    }

    pub fn take_journal(&mut self) -> Vec<Update> {
        replace(&mut self.journal, Vec::new())
    }

    pub fn chunk_structures<'a>(&'a self, chunk_id: V2) -> ChunkStructures<'a, 'd> {
        ChunkStructures {
            world: self,
            iter: self.structures_by_chunk.get(&chunk_id).map(|xs| xs.iter()),
        }
    }

    pub fn clients<'a>(&'a self) -> Clients<'a, 'd> {
        Clients {
            world: self,
            iter: self.clients.iter(),
        }
    }
}

macro_rules! lifecycle_methods {
    ($obj_ty:ty,
     $create_method:ident ( $($arg_name:ident : $arg_ty:ty),* ) => $create_op:ident
        [$ref_id_name:ident -> $ref_id_expr:expr],
     $destroy_method:ident ( $id_name:ident : $id_ty:ty ) => $destroy_op:ident) => {
        impl<'d> World<'d> {
            pub fn $create_method<'a>(&'a mut self
                                      $(, $arg_name: $arg_ty)*)
                                      -> OpResult<ObjectRefMut<'a, 'd, $obj_ty>> {
                let $ref_id_name = try!(ops::$create_op(self $(, $arg_name)*));
                Ok(ObjectRefMut {
                    world: self,
                    id: $ref_id_expr,
                })
            }

            pub fn $destroy_method(&mut self, $id_name: $id_ty) -> OpResult<()> {
                ops::$destroy_op(self, $id_name)
            }
        }
    };
    ($obj_ty:ty,
     $create_method:ident ( $($arg_name:ident : $arg_ty:ty),* ) => $create_op:ident,
     $destroy_method:ident ( $id_name:ident : $id_ty:ty ) => $destroy_op:ident) => {
        lifecycle_methods!($obj_ty,
                           $create_method($($arg_name: $arg_ty),*) => $create_op
                            [id -> id],
                           $destroy_method($id_name: $id_ty) => $destroy_op);
    };
}

lifecycle_methods!(Client,
                   create_client(chunk_offset: (u8, u8)) => client_create,
                   destroy_client(id: ClientId) => client_destroy);

lifecycle_methods!(TerrainChunk,
                   create_terrain_chunk(pos: V2, blocks: BlockChunk) => terrain_chunk_create
                    [id -> pos],
                   destroy_terrain_chunk(pos: V2) => terrain_chunk_destroy);

lifecycle_methods!(Entity,
                   create_entity(pos: V3, anim: AnimId) => entity_create,
                   destroy_entity(id: EntityId) => entity_destroy);

lifecycle_methods!(Structure,
                   create_structure(pos: V3, tid: TemplateId) => structure_create,
                   destroy_structure(id: StructureId) => structure_destroy);

macro_rules! access_methods {
    ($obj_ty:ty,
     $id_name:ident: $id_ty:ty => $table:ident . get ( $lookup_arg:expr ),
     $get_obj:ident, $get_obj_mut:ident, $obj:ident, $obj_mut:ident) => {
        impl<'d> World<'d> {
            pub fn $get_obj<'a>(&'a self,
                                $id_name: $id_ty) -> Option<ObjectRef<'a, 'd, $obj_ty>> {
                let obj = match self.$table.get($lookup_arg) {
                    None => return None,
                    Some(x) => x,
                };

                Some(ObjectRef {
                    world: self,
                    id: $id_name,
                    obj: obj,
                })
            }

            pub fn $get_obj_mut<'a>(&'a mut self,
                                    $id_name: $id_ty) -> Option<ObjectRefMut<'a, 'd, $obj_ty>> {
                match self.$table.get($lookup_arg) {
                    None => return None,
                    Some(_) => {},
                }

                Some(ObjectRefMut {
                    world: self,
                    id: $id_name,
                })
            }

            pub fn $obj<'a>(&'a self, $id_name: $id_ty) -> ObjectRef<'a, 'd, $obj_ty> {
                self.$get_obj($id_name)
                    .expect(concat!("no ", stringify!($obj_ty), " with given id"))
            }

            pub fn $obj_mut<'a>(&'a mut self, $id_name: $id_ty) -> ObjectRefMut<'a, 'd, $obj_ty> {
                self.$get_obj_mut($id_name)
                    .expect(concat!("no ", stringify!($obj_ty), " with given id"))
            }
        }
    };
}

access_methods!(Client,
                id: ClientId => clients.get(id),
                get_client, get_client_mut, client, client_mut);

access_methods!(TerrainChunk,
                id: V2 => terrain_chunks.get(&id),
                get_terrain_chunk, get_terrain_chunk_mut, terrain_chunk, terrain_chunk_mut);

access_methods!(Entity,
                id: EntityId => entities.get(id),
                get_entity, get_entity_mut, entity, entity_mut);

access_methods!(Structure,
                id: StructureId => structures.get(id),
                get_structure, get_structure_mut, structure, structure_mut);

access_methods!(Inventory,
                id: InventoryId => inventories.get(id),
                get_inventory, get_inventory_mut, inventory, inventory_mut);

pub struct ChunkStructures<'a, 'd: 'a> {
    world: &'a World<'d>,
    iter: Option<hash_set::Iter<'a, StructureId>>,
}

impl<'a, 'd> Iterator for ChunkStructures<'a, 'd> {
    type Item = ObjectRef<'a, 'd, Structure>;
    fn next(&mut self) -> Option<ObjectRef<'a, 'd, Structure>> {
        let iter = match self.iter {
            Some(ref mut x) => x,
            None => return None,
        };

        let world = self.world;
        iter.next().map(|&sid| {
            let s = &world.structures[sid];
            ObjectRef {
                world: world,
                id: sid,
                obj: s,
            }
        })
    }
}

pub struct Clients<'a, 'd: 'a> {
    world: &'a World<'d>,
    iter: StableIdMapIter<'a, ClientId, Client>,
}

impl<'a, 'd> Iterator for Clients<'a, 'd> {
    type Item = ObjectRef<'a, 'd, Client>;
    fn next(&mut self) -> Option<ObjectRef<'a, 'd, Client>> {
        let world = self.world;
        self.iter.next().map(|(cid, c)| {
            ObjectRef {
                world: world,
                id: cid,
                obj: c,
            }
        })
    }
}


impl Client {
    pub fn current_input(&self) -> InputBits {
        self.current_input
    }

    pub fn set_current_input(&mut self, new: InputBits) {
        self.current_input = new;
    }

    pub fn chunk_offset(&self) -> (u8, u8) {
        self.chunk_offset
    }

    pub fn set_chunk_offset(&mut self, new: (u8, u8)) {
        self.chunk_offset = new;
    }

    pub fn view_state(&self) -> &ViewState {
        &self.view_state
    }

    pub fn view_state_mut(&mut self) -> &mut ViewState {
        &mut self.view_state
    }
}

impl TerrainChunk {
    pub fn block(&self, idx: usize) -> BlockId {
        self.blocks[idx]
    }

    pub fn blocks(&self) -> &[BlockId; 1 << (3 * CHUNK_BITS)] {
        &self.blocks
    }
}

impl Entity {
    pub fn motion(&self) -> &Motion {
        &self.motion
    }

    // No motion_mut since modifying `self.motion` affects lookup tables.

    pub fn anim(&self) -> AnimId {
        self.anim
    }

    pub fn set_anim(&mut self, new: AnimId) {
        self.anim = new;
    }

    pub fn facing(&self) -> V3 {
        self.facing
    }

    pub fn set_facing(&mut self, new: V3) {
        self.facing = new;
    }

    pub fn pos(&self, now: Time) -> V3 {
        self.motion.pos(now)
    }

    pub fn attachment(&self) -> EntityAttachment {
        self.attachment
    }
}

impl Structure {
    pub fn pos(&self) -> V3 {
        self.pos
    }

    pub fn template_id(&self) -> TemplateId {
        self.template
    }
}


// TODO: find somewhere better to put Motion

#[derive(Clone)]
pub struct Motion {
    pub start_time: Time,
    pub duration: Duration,
    pub start_pos: V3,
    pub end_pos: V3,
}

impl Motion {
    pub fn stationary(pos: V3) -> Motion {
        Motion {
            start_time: 0,
            duration: 0,
            start_pos: pos,
            end_pos: pos,
        }
    }

    pub fn pos(&self, now: Time) -> V3 {
        if now <= self.start_time {
            self.start_pos
        } else {
            let delta = now - self.start_time;
            if delta >= self.duration as Time {
                self.end_pos
            } else {
                let offset = (self.end_pos - self.start_pos) *
                        scalar(delta as i32) / scalar(self.duration as i32);
                self.start_pos + offset
            }
        }
    }

    pub fn end_time(&self) -> Time {
        self.start_time + self.duration as Time
    }
}

