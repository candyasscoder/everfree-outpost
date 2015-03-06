use std::collections::{HashMap, hash_map, HashSet, hash_set};
use std::mem::replace;
use std::ops::{Deref, DerefMut};

use data::Data;
use input::InputBits;
use types::*;
use util::stable_id_map::{self, StableIdMap, Stable};

use self::object::{Object, ObjectRef, ObjectRefMut};
use self::hooks::{Hooks, no_hooks};
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
pub mod save;
pub mod hooks;


#[derive(Copy, PartialEq, Eq, Debug)]
pub enum EntityAttachment {
    World,
    Chunk,
    Client(ClientId),
}

#[derive(Copy, PartialEq, Eq, Debug)]
pub enum StructureAttachment {
    World,
    Chunk,
}

#[derive(Copy, PartialEq, Eq, Debug)]
pub enum InventoryAttachment {
    World,
    Client(ClientId),
    Entity(EntityId),
    Structure(StructureId),
}


pub struct Client {
    name: String,
    pawn: Option<EntityId>,
    current_input: InputBits,
    chunk_offset: (u8, u8),

    stable_id: StableId,
    child_entities: HashSet<EntityId>,
    child_inventories: HashSet<InventoryId>,
}
impl_IntrusiveStableId!(Client, stable_id);

pub struct TerrainChunk {
    blocks: Box<BlockChunk>,

    child_structures: HashSet<StructureId>,
}

pub struct Entity {
    motion: Motion,
    anim: AnimId,
    facing: V3,
    target_velocity: V3,
    appearance: u32,

    stable_id: StableId,
    attachment: EntityAttachment,
    child_inventories: HashSet<InventoryId>,
}
impl_IntrusiveStableId!(Entity, stable_id);

pub struct Structure {
    pos: V3,
    template: TemplateId,

    stable_id: StableId,
    attachment: StructureAttachment,
    child_inventories: HashSet<InventoryId>,
}
impl_IntrusiveStableId!(Structure, stable_id);

pub struct Inventory {
    contents: HashMap<ItemId, u8>,

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

pub struct WorldHooks<'a, 'd: 'a, H: Hooks+'a> {
    world: &'a mut World<'d>,
    hooks: &'a mut H,
}

impl<'a, 'd, H: Hooks> Deref for WorldHooks<'a, 'd, H> {
    type Target = World<'d>;
    fn deref(&self) -> &World<'d> {
        self.world
    }
}

impl<'a, 'd, H: Hooks> DerefMut for WorldHooks<'a, 'd, H> {
    fn deref_mut(&mut self) -> &mut World<'d> {
        self.world
    }
}

#[derive(Debug)]
pub enum Update {
    ClientCreated(ClientId),
    ClientDestroyed(ClientId),
    TerrainChunkCreated(V2),
    TerrainChunkDestroyed(V2),
    EntityCreated(EntityId),
    EntityDestroyed(EntityId),
    StructureCreated(StructureId),
    StructureDestroyed(StructureId),
    InventoryCreated(InventoryId),
    InventoryDestroyed(InventoryId),

    ClientPawnChange(ClientId),
    ChunkInvalidate(V2),
    EntityMotionChange(EntityId),
    InventoryUpdate(InventoryId, ItemId, u8, u8),

    ClientDebugInventory(ClientId, InventoryId),
    ClientOpenContainer(ClientId, InventoryId, InventoryId),
    ClientOpenCrafting(ClientId, StructureId, InventoryId),
    ClientMessage(ClientId, String),
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

    pub fn data(&self) -> &'d Data {
        self.data
    }

    pub fn hook<'a, H: Hooks>(&'a mut self, h: &'a mut H) -> WorldHooks<'a, 'd, H> {
        WorldHooks {
            world: self,
            hooks: h,
        }
    }

    pub fn record(&mut self, update: Update) {
        self.journal.push(update);
    }

    pub fn take_journal(&mut self) -> Vec<Update> {
        replace(&mut self.journal, Vec::new())
    }

    pub fn process_journal<D, F>(mut self_: D, mut f: F)
            where D: Deref<Target=World<'d>>+DerefMut,
                  F: FnMut(&mut D, Update) {
        let mut journal = replace(&mut self_.journal, Vec::new());
        for update in journal.drain() {
            f(&mut self_, update);
        }
        // Try to put back the original journal, to avoid an allocation.  But if the callback added
        // journal entries, skip it - we've already allocated, and we don't want to lose the new
        // entries.
        if self_.journal.len() == 0 {
            self_.journal = journal;
        }
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

    pub fn terrain_chunks<'a>(&'a self) -> TerrainChunksById<'a, 'd, hash_map::Keys<'a, V2, TerrainChunk>> {
        TerrainChunksById {
            world: self,
            iter: self.terrain_chunks.keys(),
        }
    }

    pub fn entities<'a>(&'a self) -> Entities<'a, 'd> {
        Entities {
            world: self,
            iter: self.entities.iter(),
        }
    }

    pub fn structures<'a>(&'a self) -> Structures<'a, 'd> {
        Structures {
            world: self,
            iter: self.structures.iter(),
        }
    }

    pub fn inventories<'a>(&'a self) -> Inventories<'a, 'd> {
        Inventories {
            world: self,
            iter: self.inventories.iter(),
        }
    }

}

macro_rules! process_objects {
    ($m:ident ! $($args:tt)*) => {
        $m!($($args)*
            object Client {
                id ClientId;
                map clients;
                lifecycle (name: &str, chunk_offset: (u8, u8))
                    create_client => client_create [id -> id],
                    destroy_client => client_destroy,
                    create_client_hooks, destroy_client_hooks;
                lookups [id -> id]
                    get_client, client,
                    get_client_mut, client_mut,
                    get_client_mut_hooks, client_mut_hooks;
                stable_ids
                    transient_client_id;
            }

            object TerrainChunk {
                id V2;
                map terrain_chunks;
                lifecycle (pos: V2, blocks: Box<BlockChunk>)
                    create_terrain_chunk => terrain_chunk_create [id -> pos],
                    destroy_terrain_chunk => terrain_chunk_destroy,
                    create_terrain_chunk_hooks, destroy_terrain_chunk_hooks;
                lookups [id -> &id]
                    get_terrain_chunk, terrain_chunk,
                    get_terrain_chunk_mut, terrain_chunk_mut,
                    get_terrain_chunk_mut_hooks, terrain_chunk_mut_hooks;
            }

            object Entity {
                id EntityId;
                map entities;
                lifecycle (pos: V3, anim: AnimId, appearance: u32)
                    create_entity => entity_create [id -> id],
                    destroy_entity => entity_destroy,
                    create_entity_hooks, destroy_entity_hooks;
                lookups [id -> id]
                    get_entity, entity,
                    get_entity_mut, entity_mut,
                    get_entity_mut_hooks, entity_mut_hooks;
                stable_ids
                    transient_entity_id;
            }

            object Structure {
                id StructureId;
                map structures;
                lifecycle (pos: V3, tid: TemplateId)
                    create_structure => structure_create [id -> id],
                    destroy_structure => structure_destroy,
                    create_structure_hooks, destroy_structure_hooks;
                lookups [id -> id]
                    get_structure, structure,
                    get_structure_mut, structure_mut,
                    get_structure_mut_hooks, structure_mut_hooks;
                stable_ids
                    transient_structure_id;
            }

            object Inventory {
                id InventoryId;
                map inventories;
                lifecycle ()
                    create_inventory => inventory_create [id -> id],
                    destroy_inventory => inventory_destroy,
                    create_inventory_hooks, destroy_inventory_hooks;
                lookups [id -> id]
                    get_inventory, inventory,
                    get_inventory_mut, inventory_mut,
                    get_inventory_mut_hooks, inventory_mut_hooks;
                stable_ids
                    transient_inventory_id;
            }
        );
    };
}

macro_rules! world_methods {
    ($(
        object $Obj:ident {
            id $Id:ident;
            map $objs:ident;
            lifecycle ($($create_arg:ident: $create_arg_ty:ty),*)
                $create_obj:ident => $create_obj_op:ident
                    [$create_id_name:ident -> $create_id_expr:expr],
                $destroy_obj:ident => $destroy_obj_op:ident,
                $create_obj_hooks:ident, $destroy_obj_hooks:ident;
            lookups [$lookup_id_name:ident -> $lookup_id_expr:expr]
                $get_obj:ident, $obj:ident,
                $get_obj_mut:ident, $obj_mut:ident,
                $get_obj_mut_hooks:ident, $obj_mut_hooks:ident;
            $(stable_ids
                $transient_obj_id:ident;)*
        }
    )*) => {
        impl<'d> World<'d> { $(
            pub fn $create_obj<'a>(&'a mut self,
                                   $($create_arg: $create_arg_ty,)*)
                                   -> OpResult<ObjectRefMut<'a, 'd, $Obj>> {
                self.$create_obj_hooks(no_hooks(), $($create_arg,)*)
            }

            pub fn $create_obj_hooks<'a, H>(&'a mut self,
                                            h: &'a mut H,
                                            $($create_arg: $create_arg_ty,)*)
                                            -> OpResult<ObjectRefMut<'a, 'd, $Obj, H>>
                    where H: Hooks {
                let $create_id_name = try!(ops::$create_obj_op(self, h, $($create_arg,)*));
                Ok(ObjectRefMut {
                    world: self,
                    hooks: h,
                    id: $create_id_expr,
                })
            }

            pub fn $destroy_obj(&mut self, id: $Id) -> OpResult<()> {
                self.$destroy_obj_hooks(no_hooks(), id)
            }

            pub fn $destroy_obj_hooks<H>(&mut self, h: &mut H, id: $Id) -> OpResult<()>
                    where H: Hooks {
                ops::$destroy_obj_op(self, h, id)
            }


            pub fn $get_obj<'a>(&'a self,
                                $lookup_id_name: $Id) -> Option<ObjectRef<'a, 'd, $Obj>> {
                let obj = match self.$objs.get($lookup_id_expr) {
                    None => return None,
                    Some(x) => x,
                };

                Some(ObjectRef {
                    world: self,
                    id: $lookup_id_name,
                    obj: obj,
                })
            }

            pub fn $obj<'a>(&'a self, id: $Id) -> ObjectRef<'a, 'd, $Obj> {
                self.$get_obj(id)
                    .expect(concat!("no ", stringify!($Obj), " with given id"))
            }

            pub fn $get_obj_mut<'a>(&'a mut self, id: $Id)
                                    -> Option<ObjectRefMut<'a, 'd, $Obj>> {
                self.$get_obj_mut_hooks(no_hooks(), id)
            }

            pub fn $obj_mut<'a>(&'a mut self, id: $Id)
                                -> ObjectRefMut<'a, 'd, $Obj> {
                self.$obj_mut_hooks(no_hooks(), id)
            }

            pub fn $get_obj_mut_hooks<'a, H>(&'a mut self,
                                             h: &'a mut H,
                                             $lookup_id_name: $Id)
                                             -> Option<ObjectRefMut<'a, 'd, $Obj, H>>
                    where H: Hooks {
                // Check that the ID is valid.
                match self.$objs.get($lookup_id_expr) {
                    None => return None,
                    Some(_) => {},
                }

                Some(ObjectRefMut {
                    world: self,
                    hooks: h,
                    id: $lookup_id_name,
                })
            }

            pub fn $obj_mut_hooks<'a, H>(&'a mut self,
                                         h: &'a mut H,
                                         id: $Id)
                                         -> ObjectRefMut<'a, 'd, $Obj, H>
                    where H: Hooks {
                self.$get_obj_mut_hooks(h, id)
                    .expect(concat!("no ", stringify!($Obj), " with given id"))
            }


            $(
                pub fn $transient_obj_id(&self, stable_id: Stable<$Id>) -> Option<$Id> {
                    self.$objs.get_id(stable_id)
                }
            )*

        )* }
    }
}

process_objects!(world_methods!);

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


macro_rules! object_iter {
    ($name:ident, $obj_ty:ty, $id_ty:ty) => {
        pub struct $name<'a, 'd: 'a> {
            world: &'a World<'d>,
            iter: stable_id_map::Iter<'a, $id_ty, $obj_ty>,
        }

        impl<'a, 'd> Iterator for $name<'a, 'd> {
            type Item = ObjectRef<'a, 'd, $obj_ty>;
            fn next(&mut self) -> Option<ObjectRef<'a, 'd, $obj_ty>> {
                let world = self.world;
                self.iter.next().map(|(oid, o)| {
                    ObjectRef {
                        world: world,
                        id: oid,
                        obj: o,
                    }
                })
            }
        }
    };
}

object_iter!(Clients, Client, ClientId);
object_iter!(Entities, Entity, EntityId);
object_iter!(Structures, Structure, StructureId);
object_iter!(Inventories, Inventory, InventoryId);


macro_rules! object_iter_by_id {
    ($name:ident, $obj_ty:ty, $id_ty:ty) => {
        pub struct $name<'a, 'd: 'a, I> {
            world: &'a World<'d>,
            iter: I,
        }

        impl<'a, 'd, 'b, I: Iterator<Item=&'b $id_ty>> Iterator for $name<'a, 'd, I> {
            type Item = ObjectRef<'a, 'd, $obj_ty>;
            fn next(&mut self) -> Option<ObjectRef<'a, 'd, $obj_ty>> {
                let world = self.world;
                self.iter.next().map(|&oid| {
                    ObjectRef {
                        world: world,
                        id: oid,
                        obj: <$obj_ty as Object>::get(world, oid).unwrap(),
                    }
                })
            }
        }
    };
}

object_iter_by_id!(ClientsById, Client, ClientId);
object_iter_by_id!(TerrainChunksById, TerrainChunk, V2);
object_iter_by_id!(EntitiesById, Entity, EntityId);
object_iter_by_id!(StructuresById, Structure, StructureId);
object_iter_by_id!(InventoriesById, Inventory, InventoryId);


/*
pub trait WorldMut<'d> {
    type Hooks: Hooks;

    fn wh_mut(&mut self) -> (&mut World<'d>, &mut Hooks);

    fn create_client<'a>(&'a mut self,
                         name: &str,
                         chunk_offset: (u8, u8))
                         -> OpResult<ObjectRefMut<'a, 'd, Client, <Self as WorldMut>::Hooks>> {
        let (w,h) = self.wh_mut();

    }
}
*/


impl Client {
    pub fn name(&self) -> &str {
        &*self.name
    }

    pub fn pawn_id(&self) -> Option<EntityId> {
        self.pawn
    }

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
}

impl TerrainChunk {
    pub fn block(&self, idx: usize) -> BlockId {
        self.blocks[idx]
    }

    pub fn blocks(&self) -> &BlockChunk {
        &*self.blocks
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

    pub fn target_velocity(&self) -> V3 {
        self.target_velocity
    }

    pub fn set_target_velocity(&mut self, new: V3) {
        self.target_velocity = new;
    }

    pub fn appearance(&self) -> u32 {
        self.appearance
    }

    pub fn set_appearance(&mut self, appearance: u32) {
        self.appearance = appearance;
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

    pub fn attachment(&self) -> StructureAttachment {
        self.attachment
    }
}

impl Inventory {
    pub fn count(&self, item_id: ItemId) -> u8 {
        self.contents.get(&item_id).map_or(0, |&x| x)
    }

    pub fn contents(&self) -> &HashMap<ItemId, u8> {
        &self.contents
    }

    pub fn attachment(&self) -> InventoryAttachment {
        self.attachment
    }
}


// TODO: find somewhere better to put Motion

#[derive(Clone, Debug)]
pub struct Motion {
    pub start_time: Time,
    pub duration: Duration,
    pub start_pos: V3,
    pub end_pos: V3,
}

impl Motion {
    pub fn fixed(pos: V3) -> Motion {
        Motion {
            start_time: 0,
            duration: 0,
            start_pos: pos,
            end_pos: pos,
        }
    }

    pub fn stationary(pos: V3, now: Time) -> Motion {
        Motion {
            start_time: now,
            duration: -1 as Duration,
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

