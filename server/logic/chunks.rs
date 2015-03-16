use std::borrow::ToOwned;
use std::error::Error;

use physics::{CHUNK_SIZE, TILE_SIZE};

use types::*;
use util::{SmallSet, SmallVec};
use util::StrResult;

use chunks;
use engine::Engine;
use engine::glue::*;
use engine::split::EngineRef;
use input::{Action, InputBits};
use messages::{ClientResponse, Dialog};
use physics_;
use script;
use terrain_gen;
use world::{self, World};
use world::object::*;
use world::save::{self, ObjectReader, ObjectWriter, ReadHooks, WriteHooks};
use vision::{self, vision_region};


pub fn load_chunk(eng: &mut Engine, cpos: V2) {
    let first = {
        let mut eng: ChunksFragment = EngineRef::new(eng).slice();
        chunks::Fragment::load(&mut eng, cpos)
    };
    if first {
        let (mut h, mut eng): (VisionHooks, _) = EngineRef::new(eng).split_off();
        eng.vision_mut().add_chunk(cpos, &mut h);
    }
}

pub fn unload_chunk(eng: &mut Engine, cpos: V2) {
    let last = {
        let mut eng: ChunksFragment = EngineRef::new(eng).slice();
        chunks::Fragment::unload(&mut eng, cpos)
    };
    if last  {
        let (mut h, mut eng): (VisionHooks, _) = EngineRef::new(eng).split_off();
        eng.vision_mut().remove_chunk(cpos, &mut h);
    }
}


impl<'a, 'd> chunks::Hooks for ChunksHooks<'a, 'd> {
}

impl<'a, 'd> chunks::Provider for ChunkProvider<'a, 'd> {
    type E = save::Error;

    fn load(&mut self, cpos: V2) -> save::Result<()> {
        if let Some(file) = self.storage().open_terrain_chunk_file(cpos) {
            let mut e: SaveReadFragment = self.borrow().slice();
            let mut sr = ObjectReader::new(file);
            try!(sr.load_terrain_chunk(&mut e));
        } else {
            let gen_chunk = {
                let mut e: TerrainGenFragment = self.borrow().slice();
                match terrain_gen::Fragment::generate(&mut e, cpos) {
                    Ok(gc) => gc,
                    Err(e) => {
                        warn!("terrain generation failed for {:?}: {}", cpos, e.description());
                        terrain_gen::GenChunk::new()
                    },
                }
            };
            {
                let mut e: HiddenWorldFragment = self.borrow().slice();
                let cid = {
                    let c = try!(world::Fragment::create_terrain_chunk(&mut e,
                                                                       cpos,
                                                                       gen_chunk.blocks));
                    c.id()
                };
                let base = cpos.extend(0) * scalar(CHUNK_SIZE);
                for gs in gen_chunk.structures.into_iter() {
                    let result = (|| {
                        let mut s = try!(world::Fragment::create_structure(&mut e,
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
        /*
        {
            let (h, e): (SaveWriteHooks, _) = self.borrow().split_off();
            let t = e.world().terrain_chunk(cpos);
            let file = e.storage().create_terrain_chunk_file(cpos);
            let mut sw = ObjectWriter::new(file, h);
            try!(sw.save_terrain_chunk(&t));
        }
        */
        {
            let mut e: HiddenWorldFragment = self.borrow().slice();
            try!(world::Fragment::destroy_terrain_chunk(&mut e, cpos));
        }
        Ok(())
    }
}
