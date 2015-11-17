use std::collections::{HashMap, HashSet};

use data::Data;
use input::InputBits;
use types::*;
use util::stable_id_map::StableIdMap;

pub use self::flags::{TerrainChunkFlags, StructureFlags};
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
pub mod flags;


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

    /// Entities indexed by their containing plane.  Entities in PLANE_LIMBO are not included here.
    entities_by_plane: HashMap<PlaneId, HashSet<EntityId>>,

    /// Entities in PLANE_LIMBO, indexed by the stable ID of their containing plane.  When a plane
    /// is loaded, its entities will automatically be moved out of limbo.
    limbo_entities: HashMap<Stable<PlaneId>, HashSet<EntityId>>,
}


pub struct Client {
    name: String,
    /// *Invariant*: If `pawn` is `Some(eid)`, then entity `eid` exists and is a child of this
    /// client.
    pawn: Option<EntityId>,
    current_input: InputBits,

    stable_id: StableId,
    child_entities: HashSet<EntityId>,
    child_inventories: HashSet<InventoryId>,
}
impl_IntrusiveStableId!(Client, stable_id);

pub struct Entity {
    /// StableId of the Plane where the Entity is currently located.
    stable_plane: Stable<PlaneId>,
    /// Cached PlaneId of the plane containing the Entity.  If that plane is not loaded, then
    /// `plane` is set to PLANE_LIMBO.
    ///
    /// *Invariant*: Either `plane` refers to a loaded plane whose stable ID is `stable_plane`, or
    /// `plane` is `PLANE_LIMBO` and the plane with stable ID `stable_plane` is not loaded.
    plane: PlaneId,

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

#[derive(Clone, Copy, Debug)]
pub enum Item {
    /// No item in this slot.
    Empty,
    /// Bulk item (stackable).  The `u8` is the item count in the stack, which should never be
    /// zero.  These items can be moved around, split, combined, etc. with no script intervention.
    Bulk(u8, ItemId),
    /// Special item (non-stackable).  This item has script data attached.  The `u8` is an
    /// identifier assigned by the script.  Moving this item to a different inventory requires
    /// script intervention.  (Moving within a container does not, because the table slot does not
    /// change.)
    Special(u8, ItemId),
}

pub struct Inventory {
    // Inventory size (number of slots) is capped at 255
    contents: Box<[Item]>,

    stable_id: StableId,
    attachment: InventoryAttachment,
}
impl_IntrusiveStableId!(Inventory, stable_id);

pub struct Plane {
    name: String,

    /// *Invariant*: If the same `cpos` is in both maps, then `saved_chunks[cpos]` is the stable ID
    /// of the chunk with ID `loaded_chunks[cpos]`.
    loaded_chunks: HashMap<V2, TerrainChunkId>,
    saved_chunks: HashMap<V2, Stable<TerrainChunkId>>,

    stable_id: StableId,
}
impl_IntrusiveStableId!(Plane, stable_id);

pub struct TerrainChunk {
    /// *Invariant*: `plane` always refers to a loaded plane.
    plane: PlaneId,
    cpos: V2,
    blocks: Box<BlockChunk>,

    stable_id: StableId,
    flags: TerrainChunkFlags,
    child_structures: HashSet<StructureId>,
}
impl_IntrusiveStableId!(TerrainChunk, stable_id);

pub struct Structure {
    /// *Invariant*: `plane` always refers to a loaded plane.
    plane: PlaneId,
    pos: V3,
    template: TemplateId,

    stable_id: StableId,
    flags: StructureFlags,
    attachment: StructureAttachment,
    child_inventories: HashSet<InventoryId>,
}
impl_IntrusiveStableId!(Structure, stable_id);

