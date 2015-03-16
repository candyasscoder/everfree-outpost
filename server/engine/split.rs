use std::marker::{PhantomData, PhantomFn};
use std::mem;

use types::*;

use data::Data;
use engine::Engine;
use storage::Storage;


macro_rules! EnginePart_decl {
    ($($tv:ident $tv2:ident $tv3:ident ($field:ident, $field_mut:ident, $fty:ty),)*) => {
        pub struct EnginePart<'a, 'd, $($tv: 'a),*> {
            ptr: *mut Engine<'d>,
            _marker0: PhantomData<&'d Data>,
            _marker1: PhantomData<&'d Storage>,
            $($field: PhantomData<&'a mut $tv>,)*
        }

        pub struct Open<'a, 'd, $($tv: 'a),*> {
            pub data: &'d Data,
            pub storage: &'d Storage,
            $( pub $field: &'a mut $tv, )*
        }

        impl<'a, 'd, $($tv),*> EnginePart<'a, 'd, $($tv),*> {
            unsafe fn from_raw(e: *mut Engine<'d>) -> EnginePart<'a, 'd, $($tv,)*> {
                EnginePart {
                    ptr: e,
                    _marker0: PhantomData,
                    _marker1: PhantomData,
                    $($field: PhantomData,)*
                }
            }

            pub fn borrow<'b>(&'b mut self) -> EnginePart<'b, 'd, $($tv),*> {
                unsafe { EnginePart::from_raw(self.ptr) }
            }

            pub fn _slice<$($tv2),*>(self) -> EnginePart<'a, 'd, $($tv2),*>
                    where EnginePart<'a, 'd, $($tv2),*>: Subpart<Self> {
                unsafe { EnginePart::from_raw(self.ptr) }
            }

            pub fn _split<$($tv2, $tv3,)*>(self) ->
                        (EnginePart<'a, 'd, $($tv2),*>,
                         EnginePart<'a, 'd, $($tv3),*>)
                    where (EnginePart<'a, 'd, $($tv2),*>,
                           EnginePart<'a, 'd, $($tv3),*>): Subpart2<Self> {
                unsafe { (EnginePart::from_raw(self.ptr),
                          EnginePart::from_raw(self.ptr)) }
            }

            pub fn _split_off<$($tv2,)*>(self) ->
                        (EnginePart<'a, 'd, $($tv2),*>,
                         EnginePart<'a, 'd, $(<$tv as SplitOffRHS<'a, $tv2>>::RHS),*>)
                    where $($tv: SplitOffRHS<'a, $tv2>,)*
                          (EnginePart<'a, 'd, $($tv2),*>,
                           EnginePart<'a, 'd, $(<$tv as SplitOffRHS<'a, $tv2>>::RHS),*>):
                              Subpart2<Self> {
                self._split()
            }

            pub fn open<'b>(&'b mut self) -> Open<'b, 'd, $($tv),*> {
                let data = self.data();
                let storage = self.storage();
                unsafe {
                    Open {
                        data: data,
                        storage: storage,
                        $( $field: mem::transmute(&mut (*self.ptr).$field), )*
                    }
                }
            }

            pub fn data(&self) -> &'d Data {
                unsafe { (*self.ptr).data }
            }

            pub fn storage(&self) -> &'d Storage {
                unsafe { (*self.ptr).storage }
            }

            pub fn now(&self) -> Time {
                unsafe { (*self.ptr).now }
            }

            $(
                pub fn $field<'b>(&'b self) -> &'b $tv {
                    unsafe {
                        mem::transmute(&(*self.ptr).$field)
                    }
                }

                pub fn $field_mut<'b>(&'b mut self) -> &'b mut $tv {
                    unsafe {
                        mem::transmute(&mut (*self.ptr).$field)
                    }
                }
            )*

            pub unsafe fn fiddle<'b: 'a>(self) -> EnginePart<'b, 'd, $($tv),*> {
                EnginePart::from_raw(self.ptr)
            }
        }


        unsafe impl<'a, 'd, $($tv, $tv2,)*> Subpart<EnginePart<'a, 'd, $($tv),*>>
                for EnginePart<'a, 'd, $($tv2),*>
                where $($tv2: Subitem<$tv>),* {}

        unsafe impl<'a, 'd, $($tv, $tv2, $tv3,)*>
                Subpart2<EnginePart<'a, 'd, $($tv),*>>
                for (EnginePart<'a, 'd, $($tv2),*>, EnginePart<'a, 'd, $($tv3),*>)
                where $(($tv2, $tv3): Subitem2<$tv>),* {}

        $( subitem_impls!($fty); )*


        pub trait Part<'a, 'd>: Sized {
            $( type $tv: 'a; )*
            fn from_part(part: EnginePart<'a, 'd, $(Self::$tv),*>) -> Self;
            fn to_part(self) -> EnginePart<'a, 'd, $(Self::$tv),*>;

            fn slice<P>(self) -> P
                    where P: Part<'a, 'd>,
                          EnginePart<'a, 'd, $(<P as Part<'a, 'd>>::$tv),*>:
                              Subpart<EnginePart<'a, 'd, $(<Self as Part<'a, 'd>>::$tv),*>> {
                P::from_part(self.to_part()._slice())
            }

            fn split_off<P>(self) -> (P, EnginePart<'a, 'd,
                // <Self::Wr as SplitOffRHS<P::Wr>>::RHS, and similarly for each other subsystem
                $(<<Self as Part<'a, 'd>>::$tv as SplitOffRHS<'a, <P as Part<'a, 'd>>::$tv>>::RHS),*
                >)
                          // P must be a Part
                    where P: Part<'a, 'd>,
                          // Self::Wr: SplitOffRHS<P::Wr>, etc.
                          $(
                            <Self as Part<'a, 'd>>::$tv: SplitOffRHS<'a, <P as Part<'a, 'd>>::$tv>,
                          )*
                          // This one basically says that self.to_part() can be split into the two
                          // pieces.
                          (EnginePart<'a, 'd, $(<P as Part<'a, 'd>>::$tv),*>, EnginePart<'a, 'd,
                           $(<<Self as Part<'a, 'd>>::$tv as SplitOffRHS<'a, <P as Part<'a, 'd>>::$tv>>::RHS),*
                           >): Subpart2<EnginePart<'a, 'd, $(<Self as Part<'a, 'd>>::$tv),*>> {
                let (a, b) = self.to_part()._split();
                (P::from_part(a), b)
            }
        }
    };
}

macro_rules! subitem_impls {
    ( $t:ty ) => {
        unsafe impl<'d> Subitem<$t> for $t {}
        unsafe impl<'d> Subitem<$t> for () {}

        unsafe impl<'d> Subitem2<$t> for ($t, ()) {}
        unsafe impl<'d> Subitem2<$t> for ((), $t) {}
        unsafe impl<'d> Subitem2<$t> for ((), ()) {}

        impl<'a, 'd: 'a> SplitOffRHS<'a, ()> for $t {
            type RHS = $t;
        }

        impl<'a, 'd: 'a> SplitOffRHS<'a, $t> for $t {
            type RHS = ();
        }
    };
}

EnginePart_decl! {
    Wr Wr2 Wr3 (world, world_mut, ::world::World<'d>),
    Sc Sc2 Sc3 (script, script_mut, ::script::ScriptEngine),
    Ms Ms2 Ms3 (messages, messages_mut, ::messages::Messages),
    Ph Ph2 Ph3 (physics, physics_mut, ::physics_::Physics<'d>),
    Vi Vi2 Vi3 (vision, vision_mut, ::vision::Vision),
    Au Au2 Au3 (auth, auth_mut, ::auth::Auth),
    Ch Ch2 Ch3 (chunks, chunks_mut, ::chunks::Chunks<'d>),
    Tg Tg2 Tg3 (terrain_gen, terrain_gen_mut, ::terrain_gen::TerrainGen<'d>),
}


unsafe trait Subpart<E>: PhantomFn<(Self, E), (Self, E)> {}

unsafe trait Subitem<A>: PhantomFn<(Self, A), (Self, A)> {}
unsafe impl Subitem<()> for () {}

unsafe trait Subpart2<E>: PhantomFn<(Self, E), (Self, E)> {}

unsafe trait Subitem2<A>: PhantomFn<(Self, A), (Self, A)> {}
unsafe impl Subitem2<()> for ((), ()) {}


trait SplitOffRHS<'a, LHS> {
    type RHS: 'a;
}

impl<'a> SplitOffRHS<'a, ()> for () {
    type RHS = ();
}


macro_rules! engine_part_typedef_helper {
    ($wr:ty, $sc:ty, $ms:ty, $ph:ty, $vi:ty, $au:ty, $ch:ty, $tg:ty, / $m:ident $name:ident /
     world, $($x:ident,)*) => {
        engine_part_typedef_helper!(
            ::world::World<'d>, $sc, $ms, $ph, $vi, $au, $ch, $tg,
            / $m $name / $($x,)*);
    };

    ($wr:ty, $sc:ty, $ms:ty, $ph:ty, $vi:ty, $au:ty, $ch:ty, $tg:ty, / $m:ident $name:ident /
     script, $($x:ident,)*) => {
        engine_part_typedef_helper!(
            $wr, ::script::ScriptEngine, $ms, $ph, $vi, $au, $ch, $tg,
            / $m $name / $($x,)*);
    };

    ($wr:ty, $sc:ty, $ms:ty, $ph:ty, $vi:ty, $au:ty, $ch:ty, $tg:ty, / $m:ident $name:ident /
     messages, $($x:ident,)*) => {
        engine_part_typedef_helper!(
            $wr, $sc, ::messages::Messages, $ph, $vi, $au, $ch, $tg,
            / $m $name / $($x,)*);
    };

    ($wr:ty, $sc:ty, $ms:ty, $ph:ty, $vi:ty, $au:ty, $ch:ty, $tg:ty, / $m:ident $name:ident /
     physics, $($x:ident,)*) => {
        engine_part_typedef_helper!(
            $wr, $sc, $ms, ::physics_::Physics<'d>, $vi, $au, $ch, $tg,
            / $m $name / $($x,)*);
    };

    ($wr:ty, $sc:ty, $ms:ty, $ph:ty, $vi:ty, $au:ty, $ch:ty, $tg:ty, / $m:ident $name:ident /
     vision, $($x:ident,)*) => {
        engine_part_typedef_helper!(
            $wr, $sc, $ms, $ph, ::vision::Vision, $au, $ch, $tg,
            / $m $name / $($x,)*);
    };

    ($wr:ty, $sc:ty, $ms:ty, $ph:ty, $vi:ty, $au:ty, $ch:ty, $tg:ty, / $m:ident $name:ident /
     auth, $($x:ident,)*) => {
        engine_part_typedef_helper!(
            $wr, $sc, $ms, $ph, $vi, ::auth::Auth, $ch, $tg,
            / $m $name / $($x,)*);
    };

    ($wr:ty, $sc:ty, $ms:ty, $ph:ty, $vi:ty, $au:ty, $ch:ty, $tg:ty, / $m:ident $name:ident /
     chunks, $($x:ident,)*) => {
        engine_part_typedef_helper!(
            $wr, $sc, $ms, $ph, $vi, $au, ::chunks::Chunks<'d>, $tg,
            / $m $name / $($x,)*);
    };

    ($wr:ty, $sc:ty, $ms:ty, $ph:ty, $vi:ty, $au:ty, $ch:ty, $tg:ty, / $m:ident $name:ident /
     terrain_gen, $($x:ident,)*) => {
        engine_part_typedef_helper!(
            $wr, $sc, $ms, $ph, $vi, $au, $ch, ::terrain_gen::TerrainGen<'d>,
            / $m $name / $($x,)*);
    };

    ($wr:ty, $sc:ty, $ms:ty, $ph:ty, $vi:ty, $au:ty, $ch:ty, $tg:ty, / $m:ident $name:ident /) => {
        $m!($name, $wr, $sc, $ms, $ph, $vi, $au, $ch, $tg);
    };
}

macro_rules! engine_part_typedef_pub {
    ($name:ident, $wr:ty, $sc:ty, $ms:ty, $ph:ty, $vi:ty, $au:ty, $ch:ty, $tg:ty) => {
        pub struct $name<'a, 'd: 'a>(pub ::engine::split::EnginePart<'a, 'd, $wr, $sc, $ms, $ph, $vi, $au, $ch, $tg>);
        engine_part_typedef_impls!($name, $wr, $sc, $ms, $ph, $vi, $au, $ch, $tg);
    };
}

macro_rules! engine_part_typedef_priv {
    ($name:ident, $wr:ty, $sc:ty, $ms:ty, $ph:ty, $vi:ty, $au:ty, $ch:ty, $tg:ty) => {
        struct $name<'a, 'd: 'a>(pub ::engine::split::EnginePart<'a, 'd, $wr, $sc, $ms, $ph, $vi, $au, $ch, $tg>);
        engine_part_typedef_impls!($name, $wr, $sc, $ms, $ph, $vi, $au, $ch, $tg);
    };
}

macro_rules! engine_part_typedef_impls {
    ($name:ident, $wr:ty, $sc:ty, $ms:ty, $ph:ty, $vi:ty, $au:ty, $ch:ty, $tg:ty) => {
        impl<'a, 'd: 'a> $crate::engine::split::Part<'a, 'd> for $name<'a, 'd> {
            type Wr = $wr;
            type Sc = $sc;
            type Ms = $ms;
            type Ph = $ph;
            type Vi = $vi;
            type Au = $au;
            type Ch = $ch;
            type Tg = $tg;

            fn from_part(part: $crate::engine::split::EnginePart<
                             'a, 'd, $wr, $sc, $ms, $ph, $vi, $au, $ch, $tg>) -> $name<'a, 'd> {
                $name(part)
            }

            fn to_part(self) -> $crate::engine::split::EnginePart<
                    'a, 'd, $wr, $sc, $ms, $ph, $vi, $au, $ch, $tg> {
                self.0
            }
        }

        impl<'a, 'd: 'a> $name<'a, 'd> {
            pub fn borrow<'b>(&'b mut self) -> $name<'b, 'd> {
                $name(self.0.borrow())
            }

            pub unsafe fn fiddle<'b: 'a>(self) -> $name<'b, 'd> {
                $name(self.0.fiddle())
            }
        }

        impl<'a, 'd: 'a> ::std::ops::Deref for $name<'a, 'd> {
            type Target = $crate::engine::split::EnginePart<
                 'a, 'd, $wr, $sc, $ms, $ph, $vi, $au, $ch, $tg>;

            fn deref(&self) -> &<Self as ::std::ops::Deref>::Target {
                &self.0
            }
        }

        impl<'a, 'd: 'a> ::std::ops::DerefMut for $name<'a, 'd> {
            fn deref_mut(&mut self) -> &mut <Self as ::std::ops::Deref>::Target {
                &mut self.0
            }
        }
    };
}

/// Macro for generating typedefs of EnginePart with only specific parts enabled (and the rest set
/// to ()).
macro_rules! engine_part_typedef {
    (pub $name:ident($($part:ident),*)) => {
        engine_part_typedef_helper!(
            (), (), (), (), (), (), (), (),
            / engine_part_typedef_pub $name / $($part,)*);
    };

    ($name:ident($($part:ident),*)) => {
        engine_part_typedef_helper!(
            (), (), (), (), (), (), (), (),
            / engine_part_typedef_priv $name / $($part,)*);
    };
}

engine_part_typedef!(pub EngineRef(world, script, messages, physics, vision, auth, chunks, terrain_gen));

impl<'a, 'd> EngineRef<'a, 'd> {
    pub fn new(e: &'a mut Engine<'d>) -> Self {
        EngineRef(unsafe { EnginePart::from_raw(e as *mut _) })
    }
}
