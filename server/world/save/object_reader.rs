use std::collections::{HashMap, HashSet};
use std::error;
use std::old_io;
use std::marker::PhantomData;
use std::mem;
use std::raw;
use std::result;

use data::Data;
use types::*;
use util::Convert;
use world::{WorldMut, Hooks};
use world::{EntityAttachment, StructureAttachment, InventoryAttachment};
use world::object::*;
use world::ops;

use super::Result;
use super::{AnyId, ToAnyId};
use super::reader::{Reader, ReaderWrapper, ReadId};
use super::CURRENT_VERSION;


pub struct ObjectReader<R: old_io::Reader, W: WorldMut, H: ReadHooks<W>> {
    r: ReaderWrapper<R>,
    hooks: H,
    template_map: HashMap<TemplateId, TemplateId>,
    item_map: HashMap<ItemId, ItemId>,
    inited_objs: HashSet<AnyId>,
    _marker0: PhantomData<fn(&mut W)>,
}

#[allow(unused_variables)]
pub trait ReadHooks<W: WorldMut> {
    fn post_read_world<R: Reader>(&mut self,
                                  reader: &mut R,
                                  w: &mut W) -> Result<()> { Ok(()) }
    fn post_read_client<R: Reader>(&mut self,
                                   reader: &mut R,
                                   w: &mut W,
                                   cid: ClientId) -> Result<()> { Ok(()) }
    fn post_read_terrain_chunk<R: Reader>(&mut self,
                                          reader: &mut R,
                                          w: &mut W,
                                          pos: V2) -> Result<()> { Ok(()) }
    fn post_read_entity<R: Reader>(&mut self,
                                   reader: &mut R,
                                   w: &mut W,
                                   eid: EntityId) -> Result<()> { Ok(()) }
    fn post_read_structure<R: Reader>(&mut self,
                                      reader: &mut R,
                                      w: &mut W,
                                      sid: StructureId) -> Result<()> { Ok(()) }
    fn post_read_inventory<R: Reader>(&mut self,
                                      reader: &mut R,
                                      w: &mut W,
                                      iid: InventoryId) -> Result<()> { Ok(()) }

    fn cleanup_world(&mut self, w: &mut W) -> Result<()> { Ok(()) }
    fn cleanup_client(&mut self, w: &mut W, cid: ClientId) -> Result<()> { Ok(()) }
    fn cleanup_terrain_chunk(&mut self, w: &mut W, pos: V2) -> Result<()> { Ok(()) }
    fn cleanup_entity(&mut self, w: &mut W, eid: EntityId) -> Result<()> { Ok(()) }
    fn cleanup_structure(&mut self, w: &mut W, sid: StructureId) -> Result<()> { Ok(()) }
    fn cleanup_inventory(&mut self, w: &mut W, iid: InventoryId) -> Result<()> { Ok(()) }
}

impl<R: old_io::Reader, W: WorldMut, H: ReadHooks<W>> ObjectReader<R, W, H> {
    pub fn new(writer: R, hooks: H) -> ObjectReader<R, W, H> {
        ObjectReader {
            r: ReaderWrapper::new(writer),
            hooks: hooks,
            template_map: HashMap::new(),
            item_map: HashMap::new(),
            inited_objs: HashSet::new(),
            _marker0: PhantomData,
        }
    }

    fn read_file_header(&mut self) -> Result<()> {
        let version: u32 = try!(self.r.read());
        if version != CURRENT_VERSION {
            fail!("file version does not match current version");
        }
        Ok(())
    }

    fn read_object_header<T: ReadId>(&mut self, w: &mut W) -> Result<(T, StableId)> {
        let id: T = try!(self.r.read_id(w));
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

        let new_id = unwrap!(data.object_templates.find_id(&*name));
        let template = data.object_templates.template(new_id);

        if template.size != size {
            fail!("template size does not match");
        }

        self.template_map.insert(old_id, new_id);
        Ok(new_id)
    }


    fn read_client(&mut self, w: &mut W) -> Result<ClientId> {
        let (cid, stable_id) = try!(self.read_object_header(w));
        try!(w.world_mut().clients.set_stable_id(cid, stable_id));

        let pawn_id = try!(self.r.read_opt_id(w));

        {
            let c = &mut w.world_mut().clients[cid];
            c.stable_id = stable_id;

            let name = try!(self.r.read_str());

            c.name = name;
            c.pawn = pawn_id;
        }
        // At this point all Client invariants hold, except that c.pawn is not yet attached to the
        // client.

        try!(self.hooks.post_read_client(&mut self.r, w, cid));

        let child_entity_count = try!(self.r.read_count());
        for _ in 0..child_entity_count {
            let eid = try!(self.read_entity(w));
            let (w,h) = w.wh_mut();
            try!(ops::entity_attach(w, h, eid, EntityAttachment::Client(cid)));
        }

        let child_inventory_count = try!(self.r.read_count());
        for _ in 0..child_inventory_count {
            let iid = try!(self.read_inventory(w));
            let (w,h) = w.wh_mut();
            try!(ops::inventory_attach(w, h, iid, InventoryAttachment::Client(cid)));
        }

        Ok(cid)
    }

    fn read_terrain_chunk(&mut self, w: &mut W) -> Result<V2> {
        let (save_id, chunk_pos) = try!(self.r.read());
        self.r.id_map_mut().insert(save_id, AnyId::TerrainChunk(chunk_pos));

        let mut blocks = Box::new([0; CHUNK_TOTAL]);
        {
            let byte_len = blocks.len() * mem::size_of::<BlockId>();
            let byte_array = unsafe {
                mem::transmute(raw::Slice {
                    data: blocks.as_ptr() as *const u8,
                    len: byte_len,
                })
            };
            try!(self.r.read_buf(byte_array));
        }

        let mut block_map = HashMap::new();
        let block_id_count = try!(self.r.read_count());
        {
            let block_data = &w.world().data().block_data;
            for _ in 0..block_id_count {
                let (old_id, shape, name_len): (u16, u8, u8) = try!(self.r.read());
                let name = try!(self.r.read_str_bytes(unwrap!(name_len.to_usize())));
                let new_id = unwrap!(block_data.find_id(&*name));

                if block_data.shape(new_id) as u8 != shape {
                    fail!("block shape does not match");
                }

                block_map.insert(old_id, new_id);
            }
        }

        for ptr in blocks.iter_mut() {
            let id = unwrap!(block_map.get(ptr));
            *ptr = *id;
        }

        {
            let (w,h) = w.wh_mut();
            try!(ops::terrain_chunk_create(w, h, chunk_pos, blocks));
        }

        try!(self.hooks.post_read_terrain_chunk(&mut self.r, w, chunk_pos));

        let child_structure_count = try!(self.r.read_count());
        for _ in 0..child_structure_count {
            let sid = try!(self.read_structure(w));
            let (w,h) = w.wh_mut();
            try!(ops::structure_attach(w, h, sid, StructureAttachment::Chunk));
        }

        Ok(chunk_pos)
    }

    fn read_entity(&mut self, w: &mut W) -> Result<EntityId> {
        let (eid, stable_id) = try!(self.read_object_header(w));
        try!(w.world_mut().entities.set_stable_id(eid, stable_id));

        {
            let e = &mut w.world_mut().entities[eid];
            e.stable_id = stable_id;

            let (start_pos,
                 end_pos,
                 start_time,
                 duration, anim,    // u16 * 2
                 facing,
                 target_velocity,
                 appearance) = try!(self.r.read());

            e.motion.start_pos = start_pos;
            e.motion.end_pos = end_pos;
            e.motion.start_time = start_time;
            e.motion.duration = duration;

            e.anim = anim;
            e.facing = facing;
            e.target_velocity = target_velocity;
            e.appearance = appearance;
        }

        {
            let (w,h) = w.wh_mut();
            w.record(::world::Update::EntityMotionChange(eid));
            h.on_entity_motion_change(w, eid);
        }

        try!(self.hooks.post_read_entity(&mut self.r, w, eid));

        let child_inventory_count = try!(self.r.read_count());
        for _ in 0..child_inventory_count {
            let iid = try!(self.read_inventory(w));
            let (w,h) = w.wh_mut();
            try!(ops::inventory_attach(w, h, iid, InventoryAttachment::Entity(eid)));
        }

        Ok(eid)
    }

    fn read_structure(&mut self, w: &mut W) -> Result<StructureId> {
        let (sid, stable_id) = try!(self.read_object_header(w));
        try!(w.world_mut().structures.set_stable_id(sid, stable_id));
        // TODO: this transmute will be unnecessary once WorldMut<'d> works
        let data = unsafe { mem::transmute::<_, &Data>(w.world().data()) };

        {
            let s = &mut w.world_mut().structures[sid];
            s.stable_id = stable_id;

            s.pos = try!(self.r.read());
            s.template = try!(self.read_template_id(data));
        }

        {
            let (w,h) = w.wh_mut();
            try!(ops::structure_post_init(w, h, sid));
        }

        try!(self.hooks.post_read_structure(&mut self.r, w, sid));

        let child_inventory_count = try!(self.r.read_count());
        for _ in 0..child_inventory_count {
            let iid = try!(self.read_inventory(w));
            let (w,h) = w.wh_mut();
            try!(ops::inventory_attach(w, h, iid, InventoryAttachment::Structure(sid)));
        }

        Ok(sid)
    }

    fn read_inventory(&mut self, w: &mut W) -> Result<InventoryId> {
        use std::collections::hash_map::Entry::*;
        let (iid, stable_id) = try!(self.read_object_header(w));
        try!(w.world_mut().inventories.set_stable_id(iid, stable_id));
        // TODO: this transmute will be unnecessary once WorldMut<'d> works
        let data = unsafe { mem::transmute::<_, &Data>(w.world().data()) };

        {
            let i = &mut w.world_mut().inventories[iid];
            i.stable_id = stable_id;

            let contents_count = try!(self.r.read_count());
            for _ in 0..contents_count {
                let (old_item_id, count, name_len): (u16, u8, u8) = try!(self.r.read());
                let item_id = match self.item_map.entry(old_item_id) {
                    Occupied(e) => *e.get(),
                    Vacant(e) => {
                        let name = try!(self.r.read_str_bytes(unwrap!(name_len.to_usize())));
                        let new_id = unwrap!(data.item_data.find_id(&*name));
                        e.insert(new_id);
                        new_id
                    },
                };

                i.contents.insert(item_id, count);
            }
        }

        try!(self.hooks.post_read_inventory(&mut self.r, w, iid));

        Ok(iid)
    }

    fn read_world(&mut self, w: &mut W) -> Result<()> {
        {
            let w = w.world_mut();
            w.clients.set_next_id(try!(self.r.read()));
            w.entities.set_next_id(try!(self.r.read()));
            w.structures.set_next_id(try!(self.r.read()));
            w.inventories.set_next_id(try!(self.r.read()));
        }

        try!(self.hooks.post_read_world(&mut self.r, w));

        {
            let entity_count = try!(self.r.read_count());
            for _ in 0..entity_count {
                try!(self.read_entity(w));
            }
        }

        {
            let structure_count = try!(self.r.read_count());
            for _ in 0..structure_count {
                try!(self.read_structure(w));
            }
        }

        {
            let inventory_count = try!(self.r.read_count());
            for _ in 0..inventory_count {
                try!(self.read_inventory(w));
            }
        }

        Ok(())
    }

    fn load_object<T, F>(&mut self, w: &mut W, f: F) -> Result<T>
            where F: FnOnce(&mut ObjectReader<R, W, H>, &mut W) -> Result<T> {
        try!(self.read_file_header());
        let result = f(self, w);
        let result = result.and_then(|x| { try!(self.check_objs()); Ok(x) });

        if result.is_err() {
            self.cleanup(w);
        }

        result
    }

    pub fn load_client(&mut self, w: &mut W) -> Result<ClientId> {
        self.load_object(w, |sr, w| sr.read_client(w))
    }

    pub fn load_terrain_chunk(&mut self, w: &mut W) -> Result<V2> {
        self.load_object(w, |sr, w| sr.read_terrain_chunk(w))
    }

    pub fn load_world(&mut self, w: &mut W) -> Result<()> {
        let result =  self.load_object(w, |sr, w| sr.read_world(w));
        if result.is_err() {
            unwrap_warn(self.hooks.cleanup_world(w));
        }
        result
    }

    fn check_objs(&mut self) -> Result<()> {
        match self.r.created_objs().difference(&self.inited_objs).next() {
            None => Ok(()),
            Some(_) => fail!("object was referenced but not defined"),
        }
    }

    fn cleanup(&mut self, w: &mut W) {
        for &aid in self.r.created_objs().iter() {
            match aid {
                AnyId::Client(cid) => {
                    unwrap_warn(self.hooks.cleanup_client(w, cid));
                    w.world_mut().clients.remove(cid);
                },
                AnyId::TerrainChunk(pos) => {
                    unwrap_warn(self.hooks.cleanup_terrain_chunk(w, pos));
                    w.world_mut().terrain_chunks.remove(&pos);
                },
                AnyId::Entity(eid) => {
                    unwrap_warn(self.hooks.cleanup_entity(w, eid));
                    w.world_mut().entities.remove(eid);
                },
                AnyId::Structure(sid) => {
                    unwrap_warn(self.hooks.cleanup_structure(w, sid));
                    {
                        let (w,h) = w.wh_mut();
                        unwrap_warn(ops::structure_pre_fini(w, h, sid));
                    }
                    w.world_mut().structures.remove(sid);
                },
                AnyId::Inventory(iid) => {
                    unwrap_warn(self.hooks.cleanup_inventory(w, iid));
                    w.world_mut().inventories.remove(iid);
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


pub struct NoReadHooks;

impl<W: WorldMut> ReadHooks<W> for NoReadHooks { }
