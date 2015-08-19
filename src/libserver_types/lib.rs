#![crate_name = "server_types"]
extern crate physics as libphysics;

use std::marker::PhantomData;
use std::{i64, u16};
use libphysics::CHUNK_BITS;

pub use libphysics::v3::{V2, V3, Vn, scalar, Region, Region2};
pub use libphysics::Shape;


// Stable IDs, for use with StableIdMap.

pub type StableId = u64;

pub const NO_STABLE_ID: StableId = 0;

/// A wrapper around StableId that prevents mixing StableIds for different types of resources.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Stable<Id> {
    pub val: StableId,
    pub _marker0: PhantomData<Id>,
}

macro_rules! const_Stable {
    ($val:expr) => {
        $crate::Stable { val: $val, _marker0: ::std::marker::PhantomData }
    };
}

impl<Id> Stable<Id> {
    pub fn none() -> Stable<Id> {
        Stable {
            val: NO_STABLE_ID,
            _marker0: PhantomData,
        }
    }

    pub fn new(val: StableId) -> Stable<Id> {
        Stable {
            val: val,
            _marker0: PhantomData,
        }
    }

    pub fn unwrap(self) -> StableId {
        self.val
    }
}


// Typedef IDs.  These are used to identify game data elements.

pub type AnimId = u16;
pub type BlockId = u16;
pub type ItemId = u16;
pub type RecipeId = u16;
pub type TileId = u16;
pub type TemplateId = u32;

// Well-known typedef ID values.
pub const EMPTY_BLOCK: BlockId = 0;
pub const PLACEHOLDER_BLOCK: BlockId = 1;


// Newtype IDs.  These are used to identify game objects (parts of the World).

macro_rules! mk_id_newtypes {
    ( $($name:ident($inner:ty);)* ) => {
        $(
            #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
            pub struct $name(pub $inner);

            impl $name {
                pub fn unwrap(self) -> $inner {
                    let $name(x) = self;
                    x
                }
            }
        )*
    };
}

mk_id_newtypes! {
    WireId(u16);
    ClientId(u16);
    EntityId(u32);
    InventoryId(u32);
    PlaneId(u32);
    TerrainChunkId(u32);
    StructureId(u32);
}

// Well-known newtype ID values.
pub const PLANE_LIMBO: PlaneId = PlaneId(0);
pub const STABLE_PLANE_LIMBO: Stable<PlaneId> = const_Stable!(1);
pub const PLANE_FOREST: PlaneId = PlaneId(1);
pub const STABLE_PLANE_FOREST: Stable<PlaneId> = const_Stable!(2);
pub const CONTROL_WIRE_ID: WireId = WireId(0);


// Time and space

pub type LocalTime = u16;
pub type LocalCoord = u16;

pub type Time = i64;
pub type Duration = u16;
pub type Coord = i32;

pub const TIME_MIN: Time = i64::MIN;
pub const TIME_MAX: Time = i64::MAX;
pub const DURATION_MIN: Duration = u16::MIN;
pub const DURATION_MAX: Duration = u16::MAX;


// Chunks

pub const CHUNK_TOTAL: usize = 1 << (3 * CHUNK_BITS);
pub type BlockChunk = [BlockId; CHUNK_TOTAL];

// 0 is always the BlockId of "empty" (no appearance; empty shape)
pub static EMPTY_CHUNK: BlockChunk = [0; CHUNK_TOTAL];
// 1 is always the BlockId of "placeholder" (no appearance; solid shape)
pub static PLACEHOLDER_CHUNK: BlockChunk = [1; CHUNK_TOTAL];
