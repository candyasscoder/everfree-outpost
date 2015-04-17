use std::error::Error;

use physics::CHUNK_SIZE;

use types::*;
use util::StringResult;

use chunks;
use engine::glue::*;
use engine::split::EngineRef;
use script;
use terrain_gen;
use world;
use world::object::*;
use world::save::{self, ObjectReader, ObjectWriter};
use vision;


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
                try!(world::Fragment::create_terrain_chunk(&mut hwf,
                                                           PLANE_FOREST,
                                                           cpos,
                                                           gen_chunk.blocks));
                let base = cpos.extend(0) * scalar(CHUNK_SIZE);
                for gs in gen_chunk.structures.into_iter() {
                    let result = (|| -> StringResult<_> {
                        let sid = {
                            let mut s = try!(world::Fragment::create_structure_unchecked(
                                    &mut hwf, gs.pos + base, gs.template));
                            s.set_attachment(world::StructureAttachment::Chunk);
                            s.id()
                        };
                        for (k, v) in gs.extra.iter() {
                            try!(script::ScriptEngine::cb_apply_structure_extra(
                                // FIXME: SUPER UNSAFE!!!  This allows scripts to violate memory
                                // safety, by mutating engine parts that are not available in the
                                // ChunpProvider fragment!
                                unsafe { ::std::mem::transmute_copy(&hwf) },
                                sid, k, v));
                        }
                        Ok(())
                    })();
                    warn_on_err!(result);
                }
            }
        }
        Ok(())
    }

    fn unload(&mut self, cpos: V2) -> save::Result<()> {
        let tcid = {
            let (h, eng) = self.borrow().0.split_off();
            let h = SaveWriteHooks(h);
            let p = eng.world().plane(PLANE_FOREST);
            let t = p.terrain_chunk(cpos);
            let file = eng.storage().create_terrain_chunk_file(cpos);
            let mut sw = ObjectWriter::new(file, h);
            try!(sw.save_terrain_chunk(&t));

            t.id()
        };
        try!(world::Fragment::destroy_terrain_chunk(&mut self.as_hidden_world_fragment(), tcid));
        Ok(())
    }
}
