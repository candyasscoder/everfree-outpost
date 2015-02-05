use std::num::{FromPrimitive, ToPrimitive};
use std::u16;
use physics::CHUNK_BITS;

pub type LocalTime = u16;
pub type LocalCoord = u16;

pub type Time = i64;
pub type Duration = u16;
pub type Coord = i32;

macro_rules! mk_id_newtypes {
    ( $($name:ident($inner:ty);)* ) => {
        $(
            #[derive(Copy, PartialEq, Eq, Hash, Show)]
            pub struct $name(pub $inner);

            impl $name {
                pub fn unwrap(self) -> $inner {
                    let $name(x) = self;
                    x
                }
            }

            impl FromPrimitive for $name {
                fn from_i64(n: i64) -> Option<$name> {
                    FromPrimitive::from_i64(n).map(|x| $name(x))
                }

                fn from_u64(n: u64) -> Option<$name> {
                    FromPrimitive::from_u64(n).map(|x| $name(x))
                }
            }

            impl ToPrimitive for $name {
                fn to_i64(&self) -> Option<i64> {
                    self.unwrap().to_i64()
                }

                fn to_u64(&self) -> Option<u64> {
                    self.unwrap().to_u64()
                }
            }
        )*
    };
}

mk_id_newtypes! {
    WireId(u16);
    ClientId(u16);
    EntityId(u32);
    StructureId(u32);
    InventoryId(u32);
}

pub type StableId = u64;

pub type AnimId = u16;
pub type BlockId = u16;
pub type ItemId = u16;
pub type TileId = u16;
pub type TemplateId = u32;

pub const DURATION_MAX: Duration = u16::MAX;

pub const CHUNK_TOTAL: usize = 1 << (3 * CHUNK_BITS);
pub type BlockChunk = [BlockId; CHUNK_TOTAL];
pub static EMPTY_CHUNK: BlockChunk = [0; CHUNK_TOTAL];


/// Trait for converting from local to global.
///
/// Converting from global coordinates to local ones throws away information.  That information is
/// recovered by consulting a "base", a global coordinate value that is known to be near the
/// original global value.  "Near" means within half the range of the local coordinate type, so if
/// local coordinates are `u16`, then "near" means within 32k (2^15) in either direction.
pub trait ToGlobal {
    type Global;
    fn to_global(self, base: <Self as ToGlobal>::Global) -> <Self as ToGlobal>::Global;
}

impl ToGlobal for LocalTime {
    type Global = Time;

    #[inline]
    fn to_global(self, base: Time) -> Time {
        let delta = self - base as u16;
        base + delta as i16 as i64
    }
}


pub trait ToLocal {
    type Local;
    fn to_local(self) -> <Self as ToLocal>::Local;
}

impl ToLocal for Time {
    type Local = LocalTime;

    #[inline(always)]
    fn to_local(self) -> LocalTime {
        self as LocalTime
    }
}
