use cache::TerrainCache;
use chunks::{self, Chunks};
use engine::split::{EngineRef, Open, Part};
use physics_::{self, Physics};
use terrain_gen::{self, TerrainGen};
use vision::{self, Vision};
use world::{self, World};


// This macro defines all the EnginePart typedefs used in the engine.  We use a macro so we can
// define parts in terms of other parts.
macro_rules! part2 {
    // Normal WorldFragment.  Changes through this fragment will be propagated to scripts, clients,
    // and the chunk cache.
    (WorldFragment, $($x:tt)*) => {
        part2!(world, WorldHooks, $($x)*);
    };
    (WorldHooks, $($x:tt)*) => {
        part2!(world, script, timer, extra, vision, cache, VisionFragment, $($x)*);
    };
    (VisionFragment, $($x:tt)*) => {
        part2!(vision, VisionHooks, $($x)*);
    };
    (VisionHooks, $($x:tt)*) => {
        part2!(world, messages, $($x)*);
    };

    // Hidden WorldFragment.  Changes through this fragment will be propagated to server data
    // structures but NOT to clients.
    (HiddenWorldFragment, $($x:tt)*) => {
        part2!(world, HiddenWorldHooks, $($x)*);
    };
    (HiddenWorldHooks, $($x:tt)*) => {
        part2!(world, script, timer, extra, cache, HiddenVisionFragment, $($x)*);
    };
    (HiddenVisionFragment, $($x:tt)*) => {
        part2!(vision, $($x)*);
    };

    // Terrain generation produces a description of the chunk, but doesn't add it to the World
    // directly.
    (TerrainGenFragment, $($x:tt)*) => {
        part2!(terrain_gen, script, HiddenWorldFragment, $($x)*);
    };

    // Chunk lifecycle events only occur on chunks that are not visible to clients, so it can use
    // HiddenWorldFragment.  This prevents a dependency cycle (WorldFragment relies on Chunks to
    // provide up-to-date block info).
    (ChunksFragment, $($x:tt)*) => {
        part2!(chunks, world, ChunkProvider, $($x)*);
    };
    (ChunkProvider, $($x:tt)*) => {
        part2!(/*HiddenWorldFragment,*/ SaveReadFragment, TerrainGenFragment, $($x)*);
    };

    (PhysicsFragment, $($x:tt)*) => {
        part2!(physics, world, cache, WorldFragment, $($x)*);
    };

    // Save read/write handling operates on HiddenWorldFragment for simplicity.  Higher-level code
    // can explicitly propagate create/destroy events to clients, 
    (SaveReadFragment, $($x:tt)*) => {
        part2!(/*HiddenWorldFragment,*/ SaveReadHooks, $($x)*);
    };
    (SaveReadHooks, $($x:tt)*) => {
        part2!(script, timer, messages, HiddenWorldFragment, $($x)*);
    };
    (SaveWriteHooks, $($x:tt)*) => {
        part2!(script, timer, messages, $($x)*);
    };


    (_done / $name:ident / $($y:ident,)*) => {
        engine_part_typedef!(pub $name($($y),*));
    };
    ($other:ident, $($x:ident),* / $name:ident / $($y:ident,)*) => {
        part2!($($x),* / $name / $($y,)* $other,);
    };
}

macro_rules! part {
    ($name:ident) => {
        part2!($name, _done / $name /);
    };
}

macro_rules! parts {
    ($($name:ident),* ,) => { parts!($($name),*) };
    ($($name:ident),*) => {
        $( part!($name); )*
    }
}


macro_rules! impl_slice {
    ($($from:ident :: $method:ident -> $to:ident;)*) => {
        $(
            impl<'a, 'd> $from<'a, 'd> {
                pub fn $method<'b>(&'b mut self) -> $to<'b, 'd> {
                    $to(self.borrow().0.slice())
                }
            }
        )*
    };
}


parts!(WorldFragment, WorldHooks, VisionFragment, VisionHooks);

impl<'a, 'd> world::Fragment<'d> for WorldFragment<'a, 'd> {
    fn world(&self) -> &World<'d> {
        (**self).world()
    }

    fn world_mut(&mut self) -> &mut World<'d> {
        (**self).world_mut()
    }

    type H = WorldHooks<'a, 'd>;
    fn with_hooks<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut WorldHooks<'a, 'd>) -> R {
        let e = unsafe { self.borrow().fiddle().to_part().slice() };
        f(&mut Part::from_part(e))
    }
}

impl<'a, 'd> vision::Fragment<'d> for VisionFragment<'a, 'd> {
    type H = VisionHooks<'a, 'd>;
    fn with_hooks<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Vision, &mut VisionHooks<'a, 'd>) -> R {
        let (h, mut e) = unsafe { self.borrow().fiddle().to_part().split_off() };
        f(e.vision_mut(), &mut Part::from_part(h))
    }
}

impl_slice! {
    EngineRef::as_world_fragment -> WorldFragment;
    EngineRef::as_vision_fragment -> VisionFragment;
    WorldHooks::as_vision_fragment -> VisionFragment;
    WorldHooks::as_hidden_world_fragment -> HiddenWorldFragment;
}


parts!(HiddenWorldFragment, HiddenWorldHooks, HiddenVisionFragment);

impl<'a, 'd> world::Fragment<'d> for HiddenWorldFragment<'a, 'd> {
    fn world(&self) -> &World<'d> {
        (**self).world()
    }

    fn world_mut(&mut self) -> &mut World<'d> {
        (**self).world_mut()
    }

    type H = HiddenWorldHooks<'a, 'd>;
    fn with_hooks<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut HiddenWorldHooks<'a, 'd>) -> R {
        let e = unsafe { self.borrow().fiddle().to_part().slice() };
        f(&mut Part::from_part(e))
    }
}

impl<'a, 'd> vision::Fragment<'d> for HiddenVisionFragment<'a, 'd> {
    type H = vision::NoHooks;
    fn with_hooks<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Vision, &mut vision::NoHooks) -> R {
        f(self.vision_mut(), &mut vision::NoHooks)
    }
}

impl_slice! {
    EngineRef::as_hidden_world_fragment -> HiddenWorldFragment;
    EngineRef::as_hidden_vision_fragment -> HiddenVisionFragment;
    HiddenWorldHooks::as_hidden_vision_fragment -> HiddenVisionFragment;
}


parts!(TerrainGenFragment);

impl<'a, 'd> terrain_gen::Fragment<'d> for TerrainGenFragment<'a, 'd> {
    fn terrain_gen_mut(&mut self) -> &mut terrain_gen::TerrainGen<'d> {
        (**self).terrain_gen_mut()
    }

    type WF = HiddenWorldFragment<'a, 'd>;
    fn with_world<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut HiddenWorldFragment<'a, 'd>) -> R {
        let e = unsafe { self.borrow().fiddle().to_part().slice() };
        f(&mut Part::from_part(e))
    }
}

impl_slice! {
    EngineRef::as_terrain_gen_fragment -> TerrainGenFragment;
}


parts!(ChunksFragment, ChunkProvider);

impl<'a, 'd> chunks::Fragment<'d> for ChunksFragment<'a, 'd> {
    fn with_world<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Chunks<'d>, &World<'d>) -> R {
        let Open { chunks, world, .. } = self.open();
        f(chunks, world)
    }

    type P = ChunkProvider<'a, 'd>;
    fn with_provider<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Chunks<'d>, &mut ChunkProvider<'a, 'd>) -> R {
        let (provider, mut e) = unsafe { self.borrow().fiddle().to_part().split_off() };
        f(e.chunks_mut(), &mut Part::from_part(provider))
    }
}

impl_slice! {
    EngineRef::as_chunks_fragment -> ChunksFragment;
    ChunkProvider::as_hidden_world_fragment -> HiddenWorldFragment;
    ChunkProvider::as_terrain_gen_fragment -> TerrainGenFragment;
    ChunkProvider::as_save_read_fragment -> SaveReadFragment;
}


parts!(PhysicsFragment);

impl<'a, 'd> physics_::Fragment<'d> for PhysicsFragment<'a, 'd> {
    fn with_cache<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Physics<'d>, &TerrainCache, &World<'d>) -> R {
        let Open { physics, cache, world, .. } = self.open();
        f(physics, cache, world)
    }

    type WF = WorldFragment<'a, 'd>;
    fn with_world<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut WorldFragment<'a, 'd>) -> R {
        let e = unsafe { self.borrow().fiddle().to_part().slice() };
        f(&mut Part::from_part(e))
    }
}

impl_slice! {
    EngineRef::as_physics_fragment -> PhysicsFragment;
}


parts!(SaveReadFragment, SaveReadHooks, SaveWriteHooks);

impl<'a, 'd> world::save::ReadFragment<'d> for SaveReadFragment<'a, 'd> {
    type WF = HiddenWorldFragment<'a, 'd>;
    fn with_world<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut HiddenWorldFragment<'a, 'd>) -> R {
        let e = unsafe { self.borrow().fiddle().to_part().slice() };
        f(&mut Part::from_part(e))
    }

    type H = SaveReadHooks<'a, 'd>;
    fn with_hooks<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut SaveReadHooks<'a, 'd>) -> R {
        let e = unsafe { self.borrow().fiddle().to_part().slice() };
        f(&mut Part::from_part(e))
    }
}

impl_slice! {
    EngineRef::as_save_read_fragment -> SaveReadFragment;
    SaveReadHooks::as_hidden_world_fragment -> HiddenWorldFragment;
}
