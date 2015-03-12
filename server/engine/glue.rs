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
use vision::Vision;
use world::{self, World};
use world::save::{self, ObjectReader, ObjectWriter};


engine_part_typedef!(pub WorldFragment(world,
                                       world, vision, messages));
engine_part_typedef!(pub WorldHooks(world, vision, messages));

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


engine_part_typedef!(pub VisionHooks(world, messages));


engine_part_typedef!(pub ChunksFragment(world, chunks,
                                        world, script, vision, messages));
engine_part_typedef!(pub ChunksHooks());
engine_part_typedef!(pub ChunkProvider(world, script, vision, messages));

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


engine_part_typedef!(pub PhysicsFragment(physics, chunks, world,
                                         world, vision, messages));

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


engine_part_typedef!(pub SaveReadFragment(world, vision, messages,
                                          script));
engine_part_typedef!(pub SaveReadHooks(script));

// NB: This typedef is the same as script::save::WriteHooks
engine_part_typedef!(pub SaveWriteHooks(script));

impl<'a, 'd> world::save::ReadFragment<'d> for SaveReadFragment<'a, 'd> {
    type WF = WorldFragment<'a, 'd>;
    fn with_world<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut WorldFragment<'a, 'd>) -> R {
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
