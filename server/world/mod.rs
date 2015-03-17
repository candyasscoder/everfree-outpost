use std::collections::{HashMap, hash_map, HashSet, hash_set};
use std::mem::{self, replace};
use std::ops::{Deref, DerefMut};

use data::Data;
use input::InputBits;
use types::*;
use util::stable_id_map::{self, StableIdMap, Stable};

use self::object::{Object, ObjectRef, ObjectRefMut};
pub use self::fragment::Fragment;
pub use self::ops::OpResult;
pub use self::hooks::Hooks;
pub use self::types::{
    EntityAttachment,
    StructureAttachment,
    InventoryAttachment,
    Motion,
};
pub use self::world::{EntitiesById, StructuresById, InventoriesById};

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

#[macro_use] pub mod world;
pub mod object;
mod ops;
mod debug;
pub mod save;
pub mod hooks;
mod types;
pub mod fragment;


// Structs must be declared at top level so that the submodules can access their private fields.

pub struct World<'d> {
    data: &'d Data,

    clients: StableIdMap<ClientId, Client>,
    terrain_chunks: HashMap<V2, TerrainChunk>,
    entities: StableIdMap<EntityId, Entity>,
    structures: StableIdMap<StructureId, Structure>,
    inventories: StableIdMap<InventoryId, Inventory>,

    structures_by_chunk: HashMap<V2, HashSet<StructureId>>,
}


pub struct Client {
    name: String,
    pawn: Option<EntityId>,
    current_input: InputBits,

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
