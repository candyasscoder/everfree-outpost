use std::collections::{HashMap, HashSet};

use data::Data;
use input::InputBits;
use types::*;
use util::stable_id_map::StableIdMap;

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
    entities: StableIdMap<EntityId, Entity>,
    inventories: StableIdMap<InventoryId, Inventory>,
    planes: StableIdMap<PlaneId, Plane>,
    terrain_chunks: StableIdMap<TerrainChunkId, TerrainChunk>,
    structures: StableIdMap<StructureId, Structure>,

    structures_by_chunk: HashMap<(PlaneId, V2), HashSet<StructureId>>,
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

pub struct Entity {
    /*
    plane: PlaneId,
    stable_plane: Stable<PlaneId>,
    */

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

pub struct Inventory {
    contents: HashMap<ItemId, u8>,

    stable_id: StableId,
    attachment: InventoryAttachment,
}
impl_IntrusiveStableId!(Inventory, stable_id);

pub struct Plane {
    loaded_chunks: HashMap<V2, TerrainChunkId>,
    saved_chunks: HashMap<V2, StableId>,

    stable_id: StableId,
}
impl_IntrusiveStableId!(Plane, stable_id);

pub struct TerrainChunk {
    plane: PlaneId,
    cpos: V2,
    blocks: Box<BlockChunk>,

    stable_id: StableId,
    child_structures: HashSet<StructureId>,
}
impl_IntrusiveStableId!(TerrainChunk, stable_id);

pub struct Structure {
    plane: PlaneId,
    pos: V3,
    template: TemplateId,

    stable_id: StableId,
    attachment: StructureAttachment,
    child_inventories: HashSet<InventoryId>,
}
impl_IntrusiveStableId!(Structure, stable_id);

