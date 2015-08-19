use std::collections::{HashMap, HashSet};
use std::error;
use std::io;
use std::mem;
use std::raw;
use std::result;

use physics::CHUNK_SIZE;
use types::*;

use data::Data;
use util::Convert;
use world;
use world::{EntityAttachment, StructureAttachment, InventoryAttachment};
use world::{TerrainChunkFlags, StructureFlags};
use world::object::*;
use world::ops;

use super::Result;
use super::{AnyId, ToAnyId};
use super::reader::{Reader, ReaderWrapper, ReadId};
use super::CURRENT_VERSION;


pub trait Fragment<'d> {
    type WF: world::Fragment<'d>;
    fn with_world<F, R>(&mut self, f: F) -> R
        where F: FnOnce(&mut Self::WF) -> R;

    type H: ReadHooks;
    fn with_hooks<F, R>(&mut self, f: F) -> R
        where F: FnOnce(&mut Self::H) -> R;
}


pub struct ObjectReader<R: io::Read> {
    r: ReaderWrapper<R>,
    file_version: u32,
    template_map: HashMap<TemplateId, TemplateId>,
    item_map: HashMap<ItemId, ItemId>,
    inited_objs: HashSet<AnyId>,
}

#[allow(unused_variables)]
pub trait ReadHooks {
    fn post_read_world<R: Reader>(&mut self,
                                  reader: &mut R) -> Result<()> { Ok(()) }
    fn post_read_client<R: Reader>(&mut self,
                                   reader: &mut R,
                                   cid: ClientId) -> Result<()> { Ok(()) }
    fn post_read_entity<R: Reader>(&mut self,
                                   reader: &mut R,
                                   eid: EntityId) -> Result<()> { Ok(()) }
    fn post_read_inventory<R: Reader>(&mut self,
                                      reader: &mut R,
                                      iid: InventoryId) -> Result<()> { Ok(()) }
    fn post_read_plane<R: Reader>(&mut self,
                                  reader: &mut R,
                                  pid: PlaneId) -> Result<()> { Ok(()) }
    fn post_read_terrain_chunk<R: Reader>(&mut self,
                                          reader: &mut R,
                                          tcid: TerrainChunkId) -> Result<()> { Ok(()) }
    fn post_read_structure<R: Reader>(&mut self,
                                      reader: &mut R,
                                      sid: StructureId,
                                      flags: StructureFlags) -> Result<()> { Ok(()) }

    fn cleanup_world(&mut self) -> Result<()> { Ok(()) }
    fn cleanup_client(&mut self, cid: ClientId) -> Result<()> { Ok(()) }
    fn cleanup_entity(&mut self, eid: EntityId) -> Result<()> { Ok(()) }
    fn cleanup_inventory(&mut self, iid: InventoryId) -> Result<()> { Ok(()) }
    fn cleanup_plane(&mut self, pid: PlaneId) -> Result<()> { Ok(()) }
    fn cleanup_terrain_chunk(&mut self, tcid: TerrainChunkId) -> Result<()> { Ok(()) }
    fn cleanup_structure(&mut self, sid: StructureId) -> Result<()> { Ok(()) }
}

impl<R: io::Read> ObjectReader<R> {
    pub fn new(reader: R) -> ObjectReader<R> {
        ObjectReader {
            r: ReaderWrapper::new(reader),
            file_version: 0,
            template_map: HashMap::new(),
            item_map: HashMap::new(),
            inited_objs: HashSet::new(),
        }
    }

    fn read_file_header(&mut self) -> Result<()> {
        let version: u32 = try!(self.r.read());
        if version != CURRENT_VERSION && version != 3 {
            fail!("file version does not match current version");
        }
        self.file_version = version;
        Ok(())
    }

    fn read_object_header<'d, T: ReadId, F: Fragment<'d>>(&mut self,
                                                          f: &mut F) -> Result<(T, StableId)> {
        let id: T = try!(f.with_world(|wf| self.r.read_id(wf)));
        let stable_id = try!(self.r.read());
        self.inited_objs.insert(id.to_any_id());
        Ok((id, stable_id))
    }

    fn read_template_id(&mut self, data: &Data) -> Result<TemplateId> {
        let old_id = try!(self.r.read());
        match self.template_map.get(&old_id) {
            Some(&new_id) => return Ok(new_id),
            None => {},
        }

        // First time seeing this ID.  Read the definition.
        let (x, y, z, name_len): (u8, u8, u8, u8) = try!(self.r.read());
        let size = V3::new(unwrap!(x.to_i32()),
                           unwrap!(y.to_i32()),
                           unwrap!(z.to_i32()));
        let name = try!(self.r.read_str_bytes(unwrap!(name_len.to_usize())));

        let new_id = unwrap!(data.structure_templates.find_id(&*name));
        let template = data.structure_templates.template(new_id);

        if template.size != size {
            fail!("template size does not match");
        }

        self.template_map.insert(old_id, new_id);
        Ok(new_id)
    }


    fn read_client<'d, F: Fragment<'d>>(&mut self,
                                        f: &mut F,
                                        name: String) -> Result<ClientId> {
        let (cid, stable_id) = try!(self.read_object_header(f));

        // TODO: check if this return type annotation is actually needed.  also check the others.
        try!(f.with_world(|wf| -> Result<_> {
            let pawn_id = try!(self.r.read_opt_id(wf));

            let w = world::Fragment::world_mut(wf);
            try!(w.clients.set_stable_id(cid, stable_id));

            let c = &mut w.clients[cid];

            c.name = name;
            c.pawn = pawn_id;
            // At this point all Client invariants hold, except that c.pawn is not yet attached to
            // the client.

            Ok(())
        }));

        try!(f.with_hooks(|h| h.post_read_client(&mut self.r, cid)));

        let child_entity_count = try!(self.r.read_count());
        for _ in 0..child_entity_count {
            let eid = try!(self.read_entity(f));
            try!(f.with_world(|wf| ops::entity::attach(wf, eid, EntityAttachment::Client(cid))));
        }

        let child_inventory_count = try!(self.r.read_count());
        for _ in 0..child_inventory_count {
            let iid = try!(self.read_inventory(f));
            try!(f.with_world(|wf|
                              ops::inventory::attach(wf, iid, InventoryAttachment::Client(cid))));
        }

        Ok(cid)
    }

    fn read_entity<'d, F: Fragment<'d>>(&mut self, f: &mut F) -> Result<EntityId> {
        let (eid, stable_id) = try!(self.read_object_header(f));

        try!(f.with_world(|wf| -> Result<_> {
            {
                let w = world::Fragment::world_mut(wf);
                try!(w.entities.set_stable_id(eid, stable_id));

                let e = &mut w.entities[eid];

                let (stable_plane,
                     start_pos,
                     end_pos,
                     start_time,
                     duration, anim,    // u16 * 2
                     facing,
                     target_velocity,
                     appearance) = try!(self.r.read());

                e.stable_plane = Stable::new(stable_plane);
                e.plane = PLANE_LIMBO;

                e.motion.start_pos = start_pos;
                e.motion.end_pos = end_pos;
                e.motion.start_time = start_time;
                e.motion.duration = duration;

                e.anim = anim;
                e.facing = facing;
                e.target_velocity = target_velocity;
                e.appearance = appearance;
            }
            ops::entity::post_init(wf, eid);
            /*
            // TODO: is it right to call hooks here?
            world::Fragment::with_hooks(wf, |h| {
                world::Hooks::on_entity_motion_change(h, eid);
            });
            */

            Ok(())
        }));

        try!(f.with_hooks(|h| h.post_read_entity(&mut self.r, eid)));

        let child_inventory_count = try!(self.r.read_count());
        for _ in 0..child_inventory_count {
            let iid = try!(self.read_inventory(f));
            try!(f.with_world(|wf|
                              ops::inventory::attach(wf, iid, InventoryAttachment::Entity(eid))));
        }

        Ok(eid)
    }

    fn read_inventory<'d, F: Fragment<'d>>(&mut self, f: &mut F) -> Result<InventoryId> {
        use std::collections::hash_map::Entry::*;
        let (iid, stable_id) = try!(self.read_object_header(f));

        try!(f.with_world(|wf| -> Result<_> {
            let w = world::Fragment::world_mut(wf);
            try!(w.inventories.set_stable_id(iid, stable_id));

            let i = &mut w.inventories[iid];

            let contents_count = try!(self.r.read_count());
            for _ in 0..contents_count {
                let (old_item_id, count, name_len): (u16, u8, u8) = try!(self.r.read());
                let item_id = match self.item_map.entry(old_item_id) {
                    Occupied(e) => *e.get(),
                    Vacant(e) => {
                        let name = try!(self.r.read_str_bytes(unwrap!(name_len.to_usize())));
                        let new_id = unwrap!(w.data.item_data.find_id(&*name));
                        e.insert(new_id);
                        new_id
                    },
                };

                i.contents.insert(item_id, count);
            }
            Ok(())
        }));

        try!(f.with_hooks(|h| h.post_read_inventory(&mut self.r, iid)));

        Ok(iid)
    }

    fn read_plane<'d, F: Fragment<'d>>(&mut self, f: &mut F) -> Result<PlaneId> {
        let (pid, stable_id) = try!(self.read_object_header(f));

        try!(f.with_world(|wf| -> Result<_> {
            {
                let w = world::Fragment::world_mut(wf);
                try!(w.planes.set_stable_id(pid, stable_id));

                let p = &mut w.planes[pid];

                p.name = try!(self.r.read_str());

                let chunks_count = try!(self.r.read_count());
                for _ in 0..chunks_count {
                    let (cpos, stable_tcid) = try!(self.r.read());
                    let stable_tcid = Stable::new(stable_tcid);
                    p.saved_chunks.insert(cpos, stable_tcid);
                }
            }
            ops::plane::post_init(wf, pid);
            Ok(())
        }));

        try!(f.with_hooks(|h| h.post_read_plane(&mut self.r, pid)));
        Ok(pid)
    }

    fn read_terrain_chunk<'d, F: Fragment<'d>>(&mut self,
                                               f: &mut F,
                                               plane: PlaneId,
                                               cpos: V2)
                                               -> Result<TerrainChunkId> {
        let (tcid, stable_id) = try!(self.read_object_header(f));

        try!(f.with_world(|wf| -> Result<()> {
            {
                let w = world::Fragment::world_mut(wf);
                try!(w.terrain_chunks.set_stable_id(tcid, stable_id));
                let data = w.data();

                let tc = &mut w.terrain_chunks[tcid];

                tc.plane = plane;
                tc.cpos = cpos;

                if self.file_version > 4 {
                    tc.flags = TerrainChunkFlags::from_bits_truncate(try!(self.r.read()));
                }

                // Read saved BlockIds into tc.blocks.
                let byte_len = tc.blocks.len() * mem::size_of::<BlockId>();
                let byte_array = unsafe {
                    mem::transmute(raw::Slice {
                        data: tc.blocks.as_ptr() as *const u8,
                        len: byte_len,
                    })
                };
                try!(self.r.read_buf(byte_array));

                // Compute block_map, mapping old BlockIds to new ones.
                let mut block_map = HashMap::new();
                let block_id_count = try!(self.r.read_count());
                let block_data = &data.block_data;
                for _ in 0..block_id_count {
                    let (old_id, shape, name_len): (u16, u8, u8) = try!(self.r.read());
                    let name = try!(self.r.read_str_bytes(unwrap!(name_len.to_usize())));
                    let new_id = unwrap!(block_data.find_id(&*name));

                    if block_data.shape(new_id) as u8 != shape {
                        fail!("block shape does not match");
                    }

                    block_map.insert(old_id, new_id);
                }

                // Replace old BlockIds with new ones in tc.blocks.
                for ptr in tc.blocks.iter_mut() {
                    let id = unwrap!(block_map.get(ptr));
                    *ptr = *id;
                }
            }

            ops::terrain_chunk::post_init(wf, tcid);
            Ok(())
        }));

        try!(f.with_hooks(|h| h.post_read_terrain_chunk(&mut self.r, tcid)));

        let child_structure_count = try!(self.r.read_count());
        let base = cpos.extend(0) * scalar(CHUNK_SIZE);
        for _ in 0..child_structure_count {
            let sid = try!(self.read_structure(f, plane, base));
            try!(f.with_world(|wf| ops::structure::attach(wf, sid, StructureAttachment::Chunk)));
        }

        Ok(tcid)
    }

    fn read_structure<'d, F: Fragment<'d>>(&mut self,
                                           f: &mut F,
                                           plane: PlaneId,
                                           base: V3)
                                           -> Result<StructureId> {
        let (sid, stable_id) = try!(self.read_object_header(f));

        let flags = try!(f.with_world(|wf| -> Result<_> {
            let flags = {
                let w = world::Fragment::world_mut(wf);
                try!(w.structures.set_stable_id(sid, stable_id));

                let s = &mut w.structures[sid];

                s.plane = plane;
                s.pos = base + try!(self.r.read());
                s.template = try!(self.read_template_id(w.data));

                if self.file_version > 3 {
                    s.flags = StructureFlags::from_bits_truncate(try!(self.r.read()));
                }

                s.flags
            };
            try!(ops::structure::post_init(wf, sid));
            Ok(flags)
        }));

        try!(f.with_hooks(|h| h.post_read_structure(&mut self.r, sid, flags)));

        let child_inventory_count = try!(self.r.read_count());
        for _ in 0..child_inventory_count {
            let iid = try!(self.read_inventory(f));
            try!(f.with_world(|wf| ops::inventory::attach(wf, iid,
                                                          InventoryAttachment::Structure(sid))));
        }

        Ok(sid)
    }

    fn read_world<'d, F: Fragment<'d>>(&mut self, f: &mut F) -> Result<()> {
        try!(f.with_world(|wf| -> Result<_> {
            let w = world::Fragment::world_mut(wf);
            w.clients.set_next_id(try!(self.r.read()));
            w.entities.set_next_id(try!(self.r.read()));
            w.inventories.set_next_id(try!(self.r.read()));
            w.planes.set_next_id(try!(self.r.read()));
            w.terrain_chunks.set_next_id(try!(self.r.read()));
            w.structures.set_next_id(try!(self.r.read()));
            Ok(())
        }));

        try!(f.with_hooks(|h| h.post_read_world(&mut self.r)));

        {
            let entity_count = try!(self.r.read_count());
            for _ in 0..entity_count {
                try!(self.read_entity(f));
            }
        }

        {
            let inventory_count = try!(self.r.read_count());
            for _ in 0..inventory_count {
                try!(self.read_inventory(f));
            }
        }

        Ok(())
    }

    fn load_object<'d, Fr: Fragment<'d>, T, F>(&mut self, frag: &mut Fr, f: F) -> Result<T>
            where F: FnOnce(&mut ObjectReader<R>, &mut Fr) -> Result<T> {
        try!(self.read_file_header());
        let result = f(self, frag);
        let result = result.and_then(|x| { try!(self.check_objs()); Ok(x) });

        if result.is_err() {
            self.cleanup(frag);
        } else {
            self.finish(frag);
        }

        result
    }

    pub fn load_client<'d, F: Fragment<'d>>(&mut self,
                                            f: &mut F,
                                            name: String) -> Result<ClientId> {
        self.load_object(f, |sr, f| sr.read_client(f, name))
    }

    pub fn load_plane<'d, F: Fragment<'d>>(&mut self, f: &mut F) -> Result<PlaneId> {
        self.load_object(f, |sr, f| sr.read_plane(f))
    }

    pub fn load_terrain_chunk<'d, F: Fragment<'d>>(&mut self,
                                                   f: &mut F,
                                                   plane: PlaneId,
                                                   cpos: V2)
                                                   -> Result<TerrainChunkId> {
        self.load_object(f, |sr, f| sr.read_terrain_chunk(f, plane, cpos))
    }

    pub fn load_world<'d, F: Fragment<'d>>(&mut self, f: &mut F) -> Result<()> {
        let result =  self.load_object(f, |sr, f| sr.read_world(f));
        if result.is_err() {
            unwrap_warn(f.with_hooks(|h| h.cleanup_world()));
        }
        result
    }

    fn check_objs(&mut self) -> Result<()> {
        match self.r.created_objs().difference(&self.inited_objs).next() {
            None => Ok(()),
            Some(_) => fail!("object was referenced but not defined"),
        }
    }

    /// Run `_create` hooks for all objects that were loaded.
    fn finish<'d, F: Fragment<'d>>(&mut self, f: &mut F) {
        use world::Fragment;
        use world::Hooks;
        for &aid in self.r.created_objs().iter() {
            match aid {
                AnyId::Client(cid) =>
                    f.with_world(|wf| wf.with_hooks(|h| h.on_client_create(cid))),
                AnyId::Entity(eid) =>
                    f.with_world(|wf| wf.with_hooks(|h| h.on_entity_create(eid))),
                AnyId::Inventory(iid) =>
                    f.with_world(|wf| wf.with_hooks(|h| h.on_inventory_create(iid))),
                AnyId::Plane(pid) =>
                    f.with_world(|wf| wf.with_hooks(|h| h.on_plane_create(pid))),
                AnyId::TerrainChunk(tcid) =>
                    f.with_world(|wf| wf.with_hooks(|h| h.on_terrain_chunk_create(tcid))),
                AnyId::Structure(sid) =>
                    f.with_world(|wf| wf.with_hooks(|h| h.on_structure_create(sid))),
            }
        }
    }

    fn cleanup<'d, F: Fragment<'d>>(&mut self, f: &mut F) {
        use world::Fragment;
        for &aid in self.r.created_objs().iter() {
            match aid {
                AnyId::Client(cid) => {
                    unwrap_warn(f.with_hooks(|h| h.cleanup_client(cid)));
                    f.with_world(|wf| wf.world_mut().clients.remove(cid));
                },
                AnyId::Entity(eid) => {
                    unwrap_warn(f.with_hooks(|h| h.cleanup_entity(eid)));
                    f.with_world(|wf| {
                        ops::entity::pre_fini(wf, eid);
                        wf.world_mut().entities.remove(eid);
                    });
                },
                AnyId::Inventory(iid) => {
                    unwrap_warn(f.with_hooks(|h| h.cleanup_inventory(iid)));
                    f.with_world(|wf| wf.world_mut().inventories.remove(iid));
                },
                AnyId::Plane(pid) => {
                    unwrap_warn(f.with_hooks(|h| h.cleanup_plane(pid)));
                    f.with_world(|wf| {
                        ops::plane::pre_fini(wf, pid);
                        wf.world_mut().planes.remove(pid);
                    });
                },
                AnyId::TerrainChunk(tcid) => {
                    unwrap_warn(f.with_hooks(|h| h.cleanup_terrain_chunk(tcid)));
                    f.with_world(|wf| {
                        ops::terrain_chunk::pre_fini(wf, tcid);
                        wf.world_mut().terrain_chunks.remove(tcid);
                    });
                },
                AnyId::Structure(sid) => {
                    unwrap_warn(f.with_hooks(|h| h.cleanup_structure(sid)));
                    f.with_world(|wf| {
                        unwrap_warn(ops::structure::pre_fini(wf, sid));
                        wf.world_mut().structures.remove(sid);
                    });
                },
            }
        }
    }
}

fn unwrap_warn<T, E: error::Error>(r: result::Result<T, E>) {
    match r {
        Ok(_) => {},
        Err(e) => warn!("error occurred during cleanup: {}",
                        error::Error::description(&e)),
    }
}
