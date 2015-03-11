use std::mem;

use types::*;

use chunks::{self, Chunks};
use data::Data;
use engine::Engine;
//use engine::logic::ChunkProvider;
use engine::split::{EngineRef, Open};
use engine::hooks::*;
use messages::Messages;
use physics_::{self, Physics};
use script::{ScriptEngine, ReadHooks, WriteHooks};
use storage::Storage;
use vision::Vision;
use world::{World, WorldMut};
use world::save::{self, ObjectReader, ObjectWriter};


impl<'a, 'd> chunks::Fragment<'d> for EngineRef<'a, 'd> {
    fn with_world<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Chunks<'d>, &World<'d>) -> R {
        let Open { chunks, world, .. } = self.open();
        f(chunks, world)
    }

    type H = Self;
    fn with_hooks<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Self) -> R {
        f(self)
    }

    type P = ChunkProvider<'a, 'd>;
    fn with_provider<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Chunks<'d>, &mut ChunkProvider<'a, 'd>) -> R {
        let (mut provider, mut e) = unsafe { self.borrow().fiddle().split_off() };
        f(e.chunks(), &mut provider)
    }
}

impl<'a, 'd> chunks::Hooks for EngineRef<'a, 'd> {
}


engine_part_typedef!(pub ChunkProvider(world, script, vision, messages));
engine_part_typedef!(ChunkProviderWS(world, script));

impl<'a, 'd> chunks::Provider for ChunkProvider<'a, 'd> {
    type E = save::Error;

    fn load(&mut self, cpos: V2) -> save::Result<()> {
        let (h, mut e) = self.borrow().split_off();
        let e = e.open();
        let mut h = WorldHooks::new(0, h);
        let mut w = e.world.hook(&mut h);

        if let Some(file) = e.storage.open_terrain_chunk_file(cpos) {
            let mut sr = ObjectReader::new(file, ReadHooks::new(e.script));
            sr.load_terrain_chunk(&mut w).unwrap();
        } else {
            let id = e.data.block_data.get_id("grass/center/v0");
            let mut blocks = [0; 4096];
            for i in range(0, 256) {
                blocks[i] = id;
            }
            w.create_terrain_chunk(cpos, Box::new(blocks)).unwrap();
        }
        Ok(())
    }

    fn unload(&mut self, cpos: V2) -> save::Result<()> {
        let (h, mut e) = self.borrow().split_off();
        let e = e.open();
        let mut h = WorldHooks::new(0, h);
        let mut w = e.world.hook(&mut h);

        {
            let t = w.world().terrain_chunk(cpos);
            let file = e.storage.create_terrain_chunk_file(cpos);
            let mut sw = ObjectWriter::new(file, WriteHooks::new(e.script));
            try!(sw.save_terrain_chunk(&t));
        }
        try!(w.destroy_terrain_chunk(cpos));
        Ok(())
    }
}


impl<'a, 'd> physics_::Fragment<'d> for EngineRef<'a, 'd> {
    fn with_chunks<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Physics<'d>, &Chunks<'d>, &World<'d>) -> R {
        let Open { physics, chunks, world, .. } = self.open();
        f(physics, chunks, world)
    }

    type WH = WorldHooks<'a, 'd>;
    fn with_world<F, R>(&mut self, f: F) -> R
            where F: for <'b> FnOnce(&mut Physics<'d>,
                                     &'b mut World<'d>,
                                     &'b mut WorldHooks<'a, 'd>) -> R {
        let (h, mut e) = unsafe { self.borrow().fiddle().split_off() };
        let e = e.open();
        let mut h = WorldHooks::new(0, h);

        f(e.physics, e.world, &mut h)
    }
}
