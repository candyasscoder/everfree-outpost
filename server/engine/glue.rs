use std::mem;

use types::*;

use chunks::{self, Chunks};
use data::Data;
use engine::Engine;
use engine::split::{EngineRef, Open};
use messages::Messages;
use physics_::{self, Physics};
use script::{ScriptEngine, ReadHooks, WriteHooks};
use storage::Storage;
use terrain_gen::{self, TerrainGen};
use vision::Vision;
use world::{self, World};
use world::save::{self, ObjectReader, ObjectWriter};


// This macro defines all the EnginePart typedefs used in the engine.  We use a macro so we can
// define parts in terms of other parts.
macro_rules! part2 {
    // Normal WorldFragment.  Changes through this fragment will be propagated to scripts and to
    // clients.
    (WorldFragment, $($x:tt)*) => {
        part2!(world, WorldHooks, $($x)*);
    };
    (WorldHooks, $($x:tt)*) => {
        part2!(script, vision, VisionHooks, $($x)*);
    };
    (VisionHooks, $($x:tt)*) => {
        part2!(world, chunks, messages, $($x)*);
    };

    // Hidden WorldFragment.  Changes through this fragment will be propagated to scripts but NOT
    // to clients.
    (HiddenWorldFragment, $($x:tt)*) => {
        part2!(world, HiddenWorldHooks, $($x)*);
    };
    (HiddenWorldHooks, $($x:tt)*) => {
        part2!(script, $($x)*);
    };

    // Terrain generation produces a description of the chunk, but doesn't add it to the World
    // directly.
    (TerrainGenFragment, $($x:tt)*) => {
        part2!(terrain_gen, script, $($x)*);
    };

    // Chunk lifecycle events only occur on chunks that are not visible to clients, so it can use
    // HiddenWorldFragment.  This prevents a dependency cycle (WorldFragment relies on Chunks to
    // provide up-to-date block info).
    (ChunksFragment, $($x:tt)*) => {
        part2!(chunks, world, ChunksHooks, ChunkProvider, $($x)*);
    };
    (ChunksHooks, $($x:tt)*) => {
        part2!($($x)*);
    };
    (ChunkProvider, $($x:tt)*) => {
        part2!(HiddenWorldFragment, SaveReadFragment, TerrainGenFragment, $($x)*);
    };

    (PhysicsFragment, $($x:tt)*) => {
        part2!(physics, chunks, world, WorldFragment, $($x)*);
    };

    // Save read/write handling operates on HiddenWorldFragment for simplicity.  Higher-level code
    // can explicitly propagate create/destroy events to clients, 
    (SaveReadFragment, $($x:tt)*) => {
        part2!(HiddenWorldFragment, SaveReadHooks, $($x)*);
    };
    (SaveReadHooks, $($x:tt)*) => {
        part2!(script, HiddenWorldFragment, $($x)*);
    };
    (SaveWriteHooks, $($x:tt)*) => {
        part2!(script, $($x)*);
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


parts!(WorldFragment, WorldHooks, VisionHooks);

impl<'a, 'd> world::Fragment<'d> for WorldFragment<'a, 'd> {
    fn world(&self) -> &World<'d> {
        self.world()
    }

    fn world_mut(&mut self) -> &mut World<'d> {
        self.world_mut()
    }

    type H = WorldHooks<'a, 'd>;
    fn with_hooks<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut WorldHooks<'a, 'd>) -> R {
        let mut e = unsafe { self.borrow().fiddle().slice() };
        f(&mut e)
    }
}


parts!(HiddenWorldFragment, HiddenWorldHooks);

impl<'a, 'd> world::Fragment<'d> for HiddenWorldFragment<'a, 'd> {
    fn world(&self) -> &World<'d> {
        self.world()
    }

    fn world_mut(&mut self) -> &mut World<'d> {
        self.world_mut()
    }

    type H = HiddenWorldHooks<'a, 'd>;
    fn with_hooks<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut HiddenWorldHooks<'a, 'd>) -> R {
        let mut e = unsafe { self.borrow().fiddle().slice() };
        f(&mut e)
    }
}


parts!(TerrainGenFragment);

impl<'a, 'd> terrain_gen::Fragment<'d> for TerrainGenFragment<'a, 'd> {
    fn open<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut TerrainGen<'d>, &mut ScriptEngine) -> R {
        let Open { terrain_gen, script, .. } = self.open();
        f(terrain_gen, script)
    }
}


parts!(ChunksFragment, ChunksHooks, ChunkProvider);

impl<'a, 'd> chunks::Fragment<'d> for ChunksFragment<'a, 'd> {
    fn with_world<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Chunks<'d>, &World<'d>) -> R {
        let Open { chunks, world, .. } = self.open();
        f(chunks, world)
    }

    type H = ChunksHooks<'a, 'd>;
    fn with_hooks<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut ChunksHooks<'a, 'd>) -> R {
        let mut e = unsafe { self.borrow().fiddle().slice() };
        f(&mut e)
    }

    type P = ChunkProvider<'a, 'd>;
    fn with_provider<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Chunks<'d>, &mut ChunkProvider<'a, 'd>) -> R {
        let (mut provider, mut e) = unsafe { self.borrow().fiddle().split_off() };
        f(e.chunks_mut(), &mut provider)
    }
}


parts!(PhysicsFragment);

impl<'a, 'd> physics_::Fragment<'d> for PhysicsFragment<'a, 'd> {
    fn with_chunks<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Physics<'d>, &Chunks<'d>, &World<'d>) -> R {
        let Open { physics, chunks, world, .. } = self.open();
        f(physics, chunks, world)
    }

    type WF = WorldFragment<'a, 'd>;
    fn with_world<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut WorldFragment<'a, 'd>) -> R {
        let mut e = unsafe { self.borrow().fiddle().slice() };
        f(&mut e)
    }
}


parts!(SaveReadFragment, SaveReadHooks, SaveWriteHooks);

impl<'a, 'd> world::save::ReadFragment<'d> for SaveReadFragment<'a, 'd> {
    type WF = HiddenWorldFragment<'a, 'd>;
    fn with_world<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut HiddenWorldFragment<'a, 'd>) -> R {
        let mut e = unsafe { self.borrow().fiddle().slice() };
        f(&mut e)
    }

    type H = SaveReadHooks<'a, 'd>;
    fn with_hooks<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut SaveReadHooks<'a, 'd>) -> R {
        let mut e = unsafe { self.borrow().fiddle().slice() };
        f(&mut e)
    }
}
