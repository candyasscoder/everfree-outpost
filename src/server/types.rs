use std::u16;
use std::i64;
use libphysics::CHUNK_BITS;

pub use libserver_types::*;


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
        let delta = self.wrapping_sub(base as u16);
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
