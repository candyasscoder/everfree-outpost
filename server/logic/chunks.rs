use std::borrow::ToOwned;
use std::error::Error;

use physics::{CHUNK_SIZE, TILE_SIZE};

use types::*;
use util::{SmallSet, SmallVec};
use util::StrResult;

use chunks;
use engine::Engine;
use engine::glue::*;
use engine::split::{EngineRef, Part};
use input::{Action, InputBits};
use messages::{ClientResponse, Dialog};
use physics_;
use script;
use terrain_gen;
use world::{self, World};
use world::object::*;
use world::save::{self, ObjectReader, ObjectWriter, ReadHooks, WriteHooks};
use vision::{self, vision_region};


pub fn load_chunk(mut eng: EngineRef, cpos: V2) {
    let first = chunks::Fragment::load(&mut eng.as_chunks_fragment(), cpos);
    if first {
        vision::Fragment::add_chunk(&mut eng.as_vision_fragment(), cpos);
    }
}

pub fn unload_chunk(mut eng: EngineRef, cpos: V2) {
    let last = chunks::Fragment::unload(&mut eng.as_chunks_fragment(), cpos);
    if last {
        vision::Fragment::remove_chunk(&mut eng.as_vision_fragment(), cpos);
    }
}


impl<'a, 'd> chunks::Hooks for ChunksHooks<'a, 'd> {
}

impl<'a, 'd> chunks::Provider for ChunkProvider<'a, 'd> {
    type E = save::Error;

    fn load(&mut self, cpos: V2) -> save::Result<()> {
        if let Some(file) = self.storage().open_terrain_chunk_file(cpos) {
            let mut sr = ObjectReader::new(file);
            try!(sr.load_terrain_chunk(&mut self.as_save_read_fragment()));
        } else {
            let gen_chunk = {
                match terrain_gen::Fragment::generate(&mut self.as_terrain_gen_fragment(), cpos) {
                    Ok(gc) => gc,
                    Err(e) => {
                        warn!("terrain generation failed for {:?}: {}", cpos, e.description());
                        terrain_gen::GenChunk::new()
                    },
                }
            };
            {
                let mut hwf = self.as_hidden_world_fragment();
                let cid = {
                    let c = try!(world::Fragment::create_terrain_chunk(&mut hwf,
                                                                       cpos,
                                                                       gen_chunk.blocks));
                    c.id()
                };
                let base = cpos.extend(0) * scalar(CHUNK_SIZE);
                for gs in gen_chunk.structures.into_iter() {
                    let result = (|| {
                        let mut s = try!(world::Fragment::create_structure(&mut hwf,
                                                                           gs.pos + base,
                                                                           gs.template));
                        s.set_attachment(world::StructureAttachment::Chunk)
                    })();
                    warn_on_err!(result);
                }
            }
        }
        Ok(())
    }

    fn unload(&mut self, cpos: V2) -> save::Result<()> {
        {
            let (h, eng) = self.borrow().0.split_off();
            let h = SaveWriteHooks(h);
            let t = eng.world().terrain_chunk(cpos);
            let file = eng.storage().create_terrain_chunk_file(cpos);
            let mut sw = ObjectWriter::new(file, h);
            try!(sw.save_terrain_chunk(&t));
        }
        try!(world::Fragment::destroy_terrain_chunk(&mut self.as_hidden_world_fragment(), cpos));
        Ok(())
    }
}
