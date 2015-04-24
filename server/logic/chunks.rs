use std::error::Error;

use physics::CHUNK_SIZE;

use types::*;
use util::StringResult;

use chunks;
use engine::glue::*;
use engine::split::EngineRef;
use script;
use terrain_gen;
use world::{self, Fragment};
use world::object::*;
use world::save::{self, ObjectReader, ObjectWriter};
use vision;


pub fn load_chunk(mut eng: EngineRef, pid: PlaneId, cpos: V2) {
    let first = chunks::Fragment::load(&mut eng.as_chunks_fragment(), pid, cpos);
    if first {
        let tcid = eng.world().plane(pid).terrain_chunk(cpos).id();
        vision::Fragment::add_terrain_chunk(&mut eng.as_vision_fragment(),
                                            tcid,
                                            pid,
                                            cpos);
    }
}

pub fn unload_chunk(mut eng: EngineRef, pid: PlaneId, cpos: V2) {
    let tcid = eng.world().plane(pid).terrain_chunk(cpos).id();
    let last = chunks::Fragment::unload(&mut eng.as_chunks_fragment(), pid, cpos);
    if last {
        vision::Fragment::remove_terrain_chunk(&mut eng.as_vision_fragment(), tcid)
    }
}


impl<'a, 'd> chunks::Provider for ChunkProvider<'a, 'd> {
    type E = save::Error;

    fn load_plane(&mut self, stable_pid: Stable<PlaneId>) -> save::Result<()> {
        let file = unwrap!(self.storage().open_plane_file(stable_pid));
        let mut sr = ObjectReader::new(file);
        try!(sr.load_plane(&mut self.as_save_read_fragment()));
        Ok(())
    }

    fn unload_plane(&mut self, pid: PlaneId) -> save::Result<()> {
        let stable_pid = self.as_hidden_world_fragment().plane_mut(pid).stable_id();
        {
            let (h, eng) = self.borrow().0.split_off();
            let h = SaveWriteHooks(h);
            let p = eng.world().plane(pid);

            let file = eng.storage().create_plane_file(stable_pid);
            let mut sw = ObjectWriter::new(file, h);
            try!(sw.save_plane(&p));
        }
        try!(world::Fragment::destroy_plane(&mut self.as_hidden_world_fragment(), pid));
        Ok(())
    }

    fn load_terrain_chunk(&mut self, pid: PlaneId, cpos: V2) -> save::Result<()> {
        // TODO(plane): use PlaneId for filename and gen
        let opt_tcid = self.world().plane(pid).get_terrain_chunk_id(cpos);
        let opt_file = opt_tcid.and_then(|tcid| self.storage().open_terrain_chunk_file(tcid));
        if let Some(file) = opt_file {
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
                                                           pid,
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

    fn unload_terrain_chunk(&mut self, pid: PlaneId, cpos: V2) -> save::Result<()> {
        // TODO(plane): use PlaneId for filename
        let tcid = {
            let (h, eng) = self.borrow().0.split_off();
            let h = SaveWriteHooks(h);
            let p = eng.world().plane(pid);
            let tc = p.terrain_chunk(cpos);

            let file = eng.storage().create_terrain_chunk_file(tc.id());
            let mut sw = ObjectWriter::new(file, h);
            try!(sw.save_terrain_chunk(&tc));

            tc.id()
        };
        try!(world::Fragment::destroy_terrain_chunk(&mut self.as_hidden_world_fragment(), tcid));
        Ok(())
    }
}
