use std::collections::{HashMap, HashSet};
use std::error;
use std::io;
use std::mem;
use std::num::ToPrimitive;
use std::raw;
use std::result;

use data::Data;
use types::*;
use world::World;
use world::{EntityAttachment, StructureAttachment, InventoryAttachment};
use world::object::*;
use world::ops;

use super::Result;
use super::{AnyId, ToAnyId};
use super::reader::{Reader, ReaderWrapper, ReadId};
use super::CURRENT_VERSION;


pub struct ObjectReader<R: io::Reader, H: ReadHooks> {
    r: ReaderWrapper<R>,
    hooks: H,
    template_map: HashMap<TemplateId, TemplateId>,
    item_map: HashMap<ItemId, ItemId>,
    inited_objs: HashSet<AnyId>,
}

#[allow(unused_variables)]
pub trait ReadHooks {
    fn post_read_world<R: Reader>(&mut self,
                                  reader: &mut R,
                                  w: &mut World) -> Result<()> { Ok(()) }
    fn post_read_client<R: Reader>(&mut self,
                                   reader: &mut R,
                                   w: &mut World,
                                   cid: ClientId) -> Result<()> { Ok(()) }
    fn post_read_terrain_chunk<R: Reader>(&mut self,
                                          reader: &mut R,
                                          w: &mut World,
                                          pos: V2) -> Result<()> { Ok(()) }
    fn post_read_entity<R: Reader>(&mut self,
                                   reader: &mut R,
                                   w: &mut World,
                                   eid: EntityId) -> Result<()> { Ok(()) }
    fn post_read_structure<R: Reader>(&mut self,
                                      reader: &mut R,
                                      w: &mut World,
                                      sid: StructureId) -> Result<()> { Ok(()) }
    fn post_read_inventory<R: Reader>(&mut self,
                                      reader: &mut R,
                                      w: &mut World,
                                      iid: InventoryId) -> Result<()> { Ok(()) }

    fn cleanup_world(&mut self, w: &mut World) -> Result<()> { Ok(()) }
    fn cleanup_client(&mut self, w: &mut World, cid: ClientId) -> Result<()> { Ok(()) }
    fn cleanup_terrain_chunk(&mut self, w: &mut World, pos: V2) -> Result<()> { Ok(()) }
    fn cleanup_entity(&mut self, w: &mut World, eid: EntityId) -> Result<()> { Ok(()) }
    fn cleanup_structure(&mut self, w: &mut World, sid: StructureId) -> Result<()> { Ok(()) }
    fn cleanup_inventory(&mut self, w: &mut World, iid: InventoryId) -> Result<()> { Ok(()) }
}

impl<R: io::Reader, H: ReadHooks> ObjectReader<R, H> {
    pub fn new(writer: R, hooks: H) -> ObjectReader<R, H> {
        ObjectReader {
            r: ReaderWrapper::new(writer),
            hooks: hooks,
            template_map: HashMap::new(),
            item_map: HashMap::new(),
            inited_objs: HashSet::new(),
        }
    }

    fn read_file_header(&mut self) -> Result<()> {
        let version: u32 = try!(self.r.read());
        if version != CURRENT_VERSION {
            fail!("file version does not match current version");
        }
        Ok(())
    }

    fn read_object_header<T: ReadId>(&mut self, w: &mut World) -> Result<(T, StableId)> {
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
        let name = try!(self.r.read_str_bytes(unwrap!(name_len.to_uint())));

        let new_id = unwrap!(data.object_templates.find_id(&*name));
        let template = data.object_templates.template(new_id);

        if template.size != size {
            fail!("template size does not match");
        }

        self.template_map.insert(old_id, new_id);
        Ok(new_id)
    }


    fn read_client(&mut self, w: &mut World) -> Result<ClientId> {
        let (cid, stable_id) = try!(self.read_object_header(w));
        try!(w.clients.set_stable_id(cid, stable_id));

        let pawn_id = try!(self.r.read_opt_id(w));

        {
            let c = &mut w.clients[cid];
            c.stable_id = stable_id;

            let name = try!(self.r.read_str());

            c.name = name;
            c.pawn = pawn_id;
        }
        // At this point all Client invariants hold, except that c.pawn is not yet attached to the
        // client.

        try!(self.hooks.post_read_client(&mut self.r, w, cid));

        let child_entity_count = try!(self.r.read_count());
        for _ in range(0, child_entity_count) {
            let eid = try!(self.read_entity(w));
            try!(ops::entity_attach(w, eid, EntityAttachment::Client(cid)));
        }

        let child_inventory_count = try!(self.r.read_count());
        for _ in range(0, child_inventory_count) {
            let iid = try!(self.read_inventory(w));
            try!(ops::inventory_attach(w, iid, InventoryAttachment::Client(cid)));
        }

        Ok(cid)
    }

    fn read_terrain_chunk(&mut self, w: &mut World) -> Result<V2> {
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
        let block_data = &w.data().block_data;
        for _ in range(0, block_id_count) {
            let (old_id, shape, name_len): (u16, u8, u8) = try!(self.r.read());
            let name = try!(self.r.read_str_bytes(unwrap!(name_len.to_uint())));
            let new_id = unwrap!(block_data.find_id(&*name));

            if block_data.shape(new_id) as u8 != shape {
                fail!("block shape does not match");
            }

            block_map.insert(old_id, new_id);
        }

        for ptr in blocks.iter_mut() {
            let id = unwrap!(block_map.get(ptr));
            *ptr = *id;
        }

        try!(ops::terrain_chunk_create(w, chunk_pos, blocks));

        try!(self.hooks.post_read_terrain_chunk(&mut self.r, w, chunk_pos));

        let child_structure_count = try!(self.r.read_count());
        for _ in range(0, child_structure_count) {
            let sid = try!(self.read_structure(w));
            try!(ops::structure_attach(w, sid, StructureAttachment::Chunk));
        }

        Ok(chunk_pos)
    }

    fn read_entity(&mut self, w: &mut World) -> Result<EntityId> {
        let (eid, stable_id) = try!(self.read_object_header(w));
        try!(w.entities.set_stable_id(eid, stable_id));

        {
            let e = &mut w.entities[eid];
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

        try!(self.hooks.post_read_entity(&mut self.r, w, eid));

        let child_inventory_count = try!(self.r.read_count());
        for _ in range(0, child_inventory_count) {
            let iid = try!(self.read_inventory(w));
            try!(ops::inventory_attach(w, iid, InventoryAttachment::Entity(eid)));
        }

        Ok(eid)
    }

    fn read_structure(&mut self, w: &mut World) -> Result<StructureId> {
        let (sid, stable_id) = try!(self.read_object_header(w));
        try!(w.structures.set_stable_id(sid, stable_id));
        let data = w.data();

        {
            let s = &mut w.structures[sid];
            s.stable_id = stable_id;

            s.pos = try!(self.r.read());
            s.template = try!(self.read_template_id(data));
        }

        try!(ops::structure_post_init(w, sid));

        try!(self.hooks.post_read_structure(&mut self.r, w, sid));

        let child_inventory_count = try!(self.r.read_count());
        for _ in range(0, child_inventory_count) {
            let iid = try!(self.read_inventory(w));
            try!(ops::inventory_attach(w, iid, InventoryAttachment::Structure(sid)));
        }

        Ok(sid)
    }

    fn read_inventory(&mut self, w: &mut World) -> Result<InventoryId> {
        use std::collections::hash_map::Entry::*;
        let (iid, stable_id) = try!(self.read_object_header(w));
        try!(w.inventories.set_stable_id(iid, stable_id));
        let data = w.data();

        {
            let i = &mut w.inventories[iid];
            i.stable_id = stable_id;

            let contents_count = try!(self.r.read_count());
            for _ in range(0, contents_count) {
                let (old_item_id, count, name_len): (u16, u8, u8) = try!(self.r.read());
                let item_id = match self.item_map.entry(old_item_id) {
                    Occupied(e) => *e.get(),
                    Vacant(e) => {
                        let name = try!(self.r.read_str_bytes(unwrap!(name_len.to_uint())));
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

    fn read_world(&mut self, w: &mut World) -> Result<()> {
        w.clients.set_next_id(try!(self.r.read()));
        w.entities.set_next_id(try!(self.r.read()));
        w.structures.set_next_id(try!(self.r.read()));
        w.inventories.set_next_id(try!(self.r.read()));

        try!(self.hooks.post_read_world(&mut self.r, w));

        {
            let entity_count = try!(self.r.read_count());
            for _ in range(0, entity_count) {
                try!(self.read_entity(w));
            }
        }

        {
            let structure_count = try!(self.r.read_count());
            for _ in range(0, structure_count) {
                try!(self.read_structure(w));
            }
        }

        {
            let inventory_count = try!(self.r.read_count());
            for _ in range(0, inventory_count) {
                try!(self.read_inventory(w));
            }
        }

        Ok(())
    }

    fn load_object<T, F>(&mut self, w: &mut World, f: F) -> Result<T>
            where F: FnOnce(&mut ObjectReader<R, H>, &mut World) -> Result<T> {
        try!(self.read_file_header());
        let result = f(self, w);
        let result = result.and_then(|x| { try!(self.check_objs()); Ok(x) });

        if result.is_err() {
            self.cleanup(w);
        }

        result
    }

    pub fn load_client(&mut self, w: &mut World) -> Result<ClientId> {
        self.load_object(w, |sr, w| sr.read_client(w))
    }

    pub fn load_terrain_chunk(&mut self, w: &mut World) -> Result<V2> {
        self.load_object(w, |sr, w| sr.read_terrain_chunk(w))
    }

    pub fn load_world(&mut self, w: &mut World) -> Result<()> {
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

    fn cleanup(&mut self, w: &mut World) {
        for &aid in self.r.created_objs().iter() {
            match aid {
                AnyId::Client(cid) => {
                    unwrap_warn(self.hooks.cleanup_client(w, cid));
                    w.clients.remove(cid);
                },
                AnyId::TerrainChunk(pos) => {
                    unwrap_warn(self.hooks.cleanup_terrain_chunk(w, pos));
                    w.terrain_chunks.remove(&pos);
                },
                AnyId::Entity(eid) => {
                    unwrap_warn(self.hooks.cleanup_entity(w, eid));
                    w.entities.remove(eid);
                },
                AnyId::Structure(sid) => {
                    unwrap_warn(self.hooks.cleanup_structure(w, sid));
                    unwrap_warn(ops::structure_pre_fini(w, sid));
                    w.structures.remove(sid);
                },
                AnyId::Inventory(iid) => {
                    unwrap_warn(self.hooks.cleanup_inventory(w, iid));
                    w.inventories.remove(iid);
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

impl ReadHooks for NoReadHooks { }
