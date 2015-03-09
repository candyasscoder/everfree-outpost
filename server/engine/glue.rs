use std::mem;

use types::*;

use chunks::{self, Chunks};
use data::Data;
use engine::Engine;
use engine::hooks::*;
use messages::Messages;
use script::{ScriptEngine, ReadHooks, WriteHooks};
use storage::Storage;
use vision::Vision;
use world::{World, WorldMut};
use world::save::{self, ObjectReader, ObjectWriter};


pub struct EngineRef<'a, 'd: 'a>(pub &'a mut Engine<'d>);

impl<'a, 'd: 'a> EngineRef<'a, 'd> {
    // Fiddle with the lifetimes a bit.  `&mut *self.0` (which is what `self.0` expands to here)
    // has type `&'b mut Engine<'d>`, where 'b is the anonymous lifetime of `self`.  We need
    // something of type `&'a mut Engine<'d>` to pass to `f`.  Note that `&mut *self.0` really does
    // live for lifetime 'a - the unsafety comes from the fact that casting the reference from 'b
    // to 'a makes it appear to the borrow checker that `self` is not borrowed.
    unsafe fn fiddle(&mut self) -> &'a mut Engine<'d> {
        fiddle(self.0)
    }
}

unsafe fn fiddle<'a: 'b, 'b, 'd: 'a>(e: &'b mut Engine<'d>) -> &'a mut Engine<'d> {
    mem::transmute(e)
}


impl<'a, 'd> chunks::Fragment<'d> for EngineRef<'a, 'd> {
    fn with_world<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Chunks<'d>, &World<'d>) -> R {
        f(&mut self.0.chunks, &self.0.world)
    }

    type H = Self;
    fn with_hooks<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Self) -> R {
        f(self)
    }

    type P = ChunkProvider<'a, 'd>;
    fn with_provider<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Chunks<'d>, &mut ChunkProvider<'a, 'd>) -> R {
        let e = unsafe { self.fiddle() };
        let mut provider = ChunkProvider {
            data: e.data,
            storage: e.storage,
            world: &mut e.world,
            script: &mut e.script,
            vision: &mut e.vision,
            messages: &mut e.messages,
        };
        f(&mut e.chunks, &mut provider)
    }
}

impl<'a, 'd> chunks::Hooks for EngineRef<'a, 'd> {
}


struct ChunkProvider<'a, 'd: 'a> {
    data: &'d Data,
    storage: &'d Storage,
    world: &'a mut World<'d>,
    script: &'a mut ScriptEngine,
    vision: &'a mut Vision,
    messages: &'a mut Messages,
}

impl<'a, 'd> chunks::Provider for ChunkProvider<'a, 'd> {
    type E = save::Error;

    fn load(&mut self, cpos: V2) -> save::Result<()> {
        let mut h = WorldHooks {
            now: 0,
            vision: self.vision,
            messages: self.messages,
        };
        let mut w = self.world.hook(&mut h);

        if let Some(file) = self.storage.open_terrain_chunk_file(cpos) {
            let mut sr = ObjectReader::new(file, ReadHooks::new(self.script));
            sr.load_terrain_chunk(&mut w).unwrap();
        } else {
            let id = self.data.block_data.get_id("grass/center/v0");
            let mut blocks = [0; 4096];
            for i in range(0, 256) {
                blocks[i] = id;
            }
            w.create_terrain_chunk(cpos, Box::new(blocks)).unwrap();
        }
        Ok(())
    }

    fn unload(&mut self, cpos: V2) -> save::Result<()> {
        {
            let t = self.world.terrain_chunk(cpos);
            let file = self.storage.create_terrain_chunk_file(cpos);
            let mut sw = ObjectWriter::new(file, WriteHooks::new(self.script));
            try!(sw.save_terrain_chunk(&t));
        }
        try!(self.world.destroy_terrain_chunk(cpos));
        Ok(())
    }
}

