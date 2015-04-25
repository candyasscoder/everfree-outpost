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
    trace!("load_chunk({:?}, {:?})", pid, cpos);
    chunks::Fragment::load(&mut eng.as_chunks_fragment(), pid, cpos);
}

pub fn unload_chunk(mut eng: EngineRef, pid: PlaneId, cpos: V2) {
    trace!("unload_chunk({:?}, {:?})", pid, cpos);
    let tcid = eng.world().plane(pid).terrain_chunk(cpos).id();
    chunks::Fragment::unload(&mut eng.as_chunks_fragment(), pid, cpos);
}

// NB: This should only be used when there is reason to believe none of the plane's chunks are
// loaded.
pub fn unload_plane(mut eng: EngineRef, pid: PlaneId) {
    trace!("unload_plane({:?})", pid);
    chunks::Fragment::unload_plane(&mut eng.as_chunks_fragment(), pid);
}


impl<'a, 'd> chunks::Provider for ChunkProvider<'a, 'd> {
    type E = save::Error;

    fn load_plane(&mut self, stable_pid: Stable<PlaneId>) -> save::Result<()> {
        trace!("load_plane({:?})", stable_pid);
        let file = unwrap!(self.storage().open_plane_file(stable_pid));
        let mut sr = ObjectReader::new(file);
        try!(sr.load_plane(&mut self.as_save_read_fragment()));
        Ok(())
    }

    fn unload_plane(&mut self, pid: PlaneId) -> save::Result<()> {
        let stable_pid = self.as_hidden_world_fragment().plane_mut(pid).stable_id();
        trace!("unload_plane({:?})", stable_pid);
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
        trace!("load_terrain_chunk({:?}, {:?})", pid, cpos);
        let opt_tcid = self.world().plane(pid).get_saved_terrain_chunk_id(cpos);
        let opt_file = opt_tcid.and_then(|tcid| self.storage().open_terrain_chunk_file(tcid));
        if let Some(file) = opt_file {
            let mut sr = ObjectReader::new(file);
            try!(sr.load_terrain_chunk(&mut self.as_save_read_fragment(), pid, cpos));
        } else {
            trace!("generating terrain for {:?} {:?}", pid, cpos);
            let gen_chunk = {
                let stable_pid = self.as_hidden_world_fragment().plane_mut(pid).stable_id();
                let (tgf, eng) = self.borrow().0.split_off();
                let result = terrain_gen::Fragment::generate(&mut TerrainGenFragment(tgf),
                                                             stable_pid,
                                                             eng.world().plane(pid).name(),
                                                             cpos);
                match result {
                    Ok(gc) => gc,
                    Err(e) => {
                        warn!("terrain generation failed for {:?} {:?}: {}",
                              pid, cpos, e.description());
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
                                    &mut hwf, pid, gs.pos + base, gs.template));
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
        trace!("unload_terrain_chunk({:?}, {:?})", pid, cpos);
        // TODO(plane): use PlaneId for filename
        let stable_tcid = self.as_hidden_world_fragment().plane_mut(pid).save_terrain_chunk(cpos);
        let tcid = {
            let (h, eng) = self.borrow().0.split_off();
            let h = SaveWriteHooks(h);
            let p = eng.world().plane(pid);
            let tc = p.terrain_chunk(cpos);

            let file = eng.storage().create_terrain_chunk_file(stable_tcid);
            let mut sw = ObjectWriter::new(file, h);
            try!(sw.save_terrain_chunk(&tc));

            tc.id()
        };
        trace!("unload_terrain_chunk({:?}, {:?}): tcid = {:?}", pid, cpos, tcid);
        try!(world::Fragment::destroy_terrain_chunk(&mut self.as_hidden_world_fragment(), tcid));
        Ok(())
    }
}
