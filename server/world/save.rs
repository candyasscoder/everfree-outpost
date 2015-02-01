use std::collections::{HashMap, HashSet};
use std::error;
use std::io;
use std::mem;
use std::num::ToPrimitive;
use std::raw;
use std::result;

use physics::v3::{Vn, V3, V2};

use data::Data;
use types::*;
use util::IntrusiveStableId;
use util::StrError;
use world::{World, Client, TerrainChunk, Entity, Structure, Inventory};
use world::{EntityAttachment, StructureAttachment};
use world::ops;
use world::object::*;

type SaveId = u32;

#[derive(Copy, PartialEq, Eq, Show, Hash)]
enum AnyId {
    Client(ClientId),
    TerrainChunk(V2),
    Entity(EntityId),
    Structure(StructureId),
    Inventory(InventoryId),
}

trait ToAnyId {
    fn to_any_id(self) -> AnyId;
}

impl ToAnyId for AnyId {
    fn to_any_id(self) -> AnyId { self }
}

impl ToAnyId for ClientId {
    fn to_any_id(self) -> AnyId { AnyId::Client(self) }
}

impl ToAnyId for EntityId {
    fn to_any_id(self) -> AnyId { AnyId::Entity(self) }
}

impl ToAnyId for StructureId {
    fn to_any_id(self) -> AnyId { AnyId::Structure(self) }
}

impl ToAnyId for InventoryId {
    fn to_any_id(self) -> AnyId { AnyId::Inventory(self) }
}


trait ReadId: ToAnyId+Copy {
    fn from_any_id(id: AnyId) -> Result<Self>;
    fn fabricate(w: &mut World) -> Self;
}

impl ReadId for ClientId {
    fn from_any_id(id: AnyId) -> Result<ClientId> {
        match id {
            AnyId::Client(id) => Ok(id),
            _ => fail!("expected AnyID::Client"),
        }
    }

    fn fabricate(w: &mut World) -> ClientId {
        ops::client_create_unchecked(w)
    }
}

impl ReadId for EntityId {
    fn from_any_id(id: AnyId) -> Result<EntityId> {
        match id {
            AnyId::Entity(id) => Ok(id),
            _ => fail!("expected AnyID::Entity"),
        }
    }

    fn fabricate(w: &mut World) -> EntityId {
        ops::entity_create_unchecked(w)
    }
}

impl ReadId for StructureId {
    fn from_any_id(id: AnyId) -> Result<StructureId> {
        match id {
            AnyId::Structure(id) => Ok(id),
            _ => fail!("expected AnyID::Structure"),
        }
    }

    fn fabricate(w: &mut World) -> StructureId {
        ops::structure_create_unchecked(w)
    }
}

impl ReadId for InventoryId {
    fn from_any_id(id: AnyId) -> Result<InventoryId> {
        match id {
            AnyId::Inventory(id) => Ok(id),
            _ => fail!("expected AnyID::Inventory"),
        }
    }

    fn fabricate(w: &mut World) -> InventoryId {
        ops::inventory_create_unchecked(w)
    }
}


#[derive(Show)]
pub enum Error {
    Io(io::IoError),
    Str(StrError),
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref e) => e.description(),
            Error::Str(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Io(ref e) => Some(e as &error::Error),
            Error::Str(ref e) => Some(e as &error::Error),
        }
    }
}

impl error::FromError<io::IoError> for Error {
    fn from_error(e: io::IoError) -> Error {
        Error::Io(e)
    }
}

impl error::FromError<StrError> for Error {
    fn from_error(e: StrError) -> Error {
        Error::Str(e)
    }
}

pub type Result<T> = result::Result<T, Error>;


pub struct SaveWriter<W: Writer> {
    writer: W,

    id_map: HashMap<AnyId, SaveId>,
    next_id: SaveId,
    objects_written: HashSet<AnyId>,
    seen_templates: HashSet<TemplateId>,
}

const CURRENT_VERSION: u32 = 1;

impl<W: Writer> SaveWriter<W> {
    pub fn new(writer: W) -> SaveWriter<W> {
        SaveWriter {
            writer: writer,
            id_map: HashMap::new(),
            next_id: 0,
            objects_written: HashSet::new(),
            seen_templates: HashSet::new(),
        }
    }

    fn write_header(&mut self) -> Result<()> {
        try!(self.writer.write_le_u32(CURRENT_VERSION));
        Ok(())
    }


    fn write_id<T: ToAnyId>(&mut self, id: T) -> Result<()> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        let id = id.to_any_id();
        let save_id = match self.id_map.entry(id) {
            Occupied(e) => *e.get(),
            Vacant(e) => {
                let sid = e.insert(self.next_id);
                self.next_id += 1;
                *sid
            },
        };
        try!(self.writer.write_le_u32(save_id));
        Ok(())
    }

    fn write_opt_id<T: ToAnyId>(&mut self, opt_id: Option<T>) -> Result<()> {
        match opt_id {
            Some(id) => try!(self.write_id(id)),
            None => try!(self.writer.write_le_u32(-1 as SaveId)),
        }
        Ok(())
    }

    fn write_object_header<O>(&mut self, o: &ObjectRef<O>) -> Result<()>
            where O: Object+IntrusiveStableId,
                  <O as Object>::Id: ToAnyId {
        // NB: write_terrain_chunk contains a specialized copy of this code that can deal with
        // TerrainChunk's idiosyncrasies.
        try!(self.write_id(o.id()));
        try!(self.writer.write_le_u64(o.get_stable_id()));
        self.objects_written.insert(o.id().to_any_id());
        Ok(())
    }

    fn write_count(&mut self, count: usize) -> Result<()> {
        try!(self.writer.write_le_u32(unwrap!(count.to_u32())));
        Ok(())
    }

    fn write_v2(&mut self, v: V2) -> Result<()> {
        try!(self.writer.write_le_i32(v.x));
        try!(self.writer.write_le_i32(v.y));
        Ok(())
    }

    fn write_v3(&mut self, v: V3) -> Result<()> {
        try!(self.writer.write_le_i32(v.x));
        try!(self.writer.write_le_i32(v.y));
        try!(self.writer.write_le_i32(v.z));
        Ok(())
    }

    fn write_str_bytes(&mut self, s: &str) -> Result<()> {
        try!(self.writer.write(s.as_bytes()));
        let padding = (4 - (s.len() % 4)) % 4;
        try!(self.writer.write([0; 3].slice_to(padding)));
        Ok(())
    }

    fn write_template_id(&mut self, data: &Data, template_id: TemplateId) -> Result<()> {
        try!(self.writer.write_le_u32(template_id));
        if !self.seen_templates.contains(&template_id) {
            self.seen_templates.insert(template_id);

            let template = data.object_templates.template(template_id);
            try!(self.writer.write_u8(unwrap!(template.size.x.to_u8())));
            try!(self.writer.write_u8(unwrap!(template.size.y.to_u8())));
            try!(self.writer.write_u8(unwrap!(template.size.z.to_u8())));
            try!(self.writer.write_u8(unwrap!(template.name.len().to_u8())));
            try!(self.write_str_bytes(&*template.name));
        }
        Ok(())
    }


    fn write_client(&mut self, c: &ObjectRef<Client>) -> Result<()> {
        try!(self.write_object_header(c));

        // Body
        try!(self.write_opt_id(c.pawn));
        try!(self.write_count(c.name().len()));
        try!(self.write_str_bytes(c.name()));

        // Children
        try!(self.write_count(c.child_entities.len()));
        for e in c.child_entities() {
            try!(self.write_entity(&e));
        }

        try!(self.write_count(c.child_inventories.len()));
        for i in c.child_inventories() {
            try!(self.write_inventory(&i));
        }

        Ok(())
    }

    fn write_terrain_chunk(&mut self, t: &ObjectRef<TerrainChunk>) -> Result<()> {
        // Write a custom object header.  Assign it a SaveId like normal, but use the chunk
        // position as a stable ID.
        let aid = AnyId::TerrainChunk(t.id());
        try!(self.write_id(aid));
        try!(self.write_v2(t.id()));
        self.objects_written.insert(aid);

        // Body - block data
        let len = t.blocks.len();
        let byte_len = len * mem::size_of::<BlockId>();
        let byte_array = unsafe {
            mem::transmute(raw::Slice {
                data: t.blocks.as_ptr() as *const u8,
                len: byte_len,
            })
        };
        assert!(byte_len % 4 == 0);
        try!(self.writer.write(byte_array));

        // Body - lookup table
        let blocks_seen = t.blocks.iter().map(|&x| x).collect::<HashSet<_>>();
        try!(self.write_count(blocks_seen.len()));
        let block_data = &t.world().data().block_data;
        for b in blocks_seen.into_iter() {
            let shape = block_data.shape(b);
            let name = block_data.name(b);
            try!(self.writer.write_le_u16(b));
            try!(self.writer.write_u8(shape as u8));
            try!(self.writer.write_u8(unwrap!(name.len().to_u8())));
            try!(self.write_str_bytes(name));
        }

        // Children
        try!(self.write_count(t.child_structures.len()));
        for s in t.child_structures() {
            try!(self.write_structure(&s));
        }

        Ok(())
    }

    fn write_entity(&mut self, e: &ObjectRef<Entity>) -> Result<()> {
        try!(self.write_object_header(e));

        // Body
        let m = &e.motion;
        try!(self.write_v3(m.start_pos));
        try!(self.write_v3(m.end_pos));
        try!(self.writer.write_le_i64(m.start_time));
        // NB: Pack 16-bit items m.duration and e.anim together.
        try!(self.writer.write_le_u16(m.duration));

        try!(self.writer.write_le_u16(e.anim));
        try!(self.write_v3(e.facing));
        try!(self.write_v3(e.target_velocity));

        // Children
        try!(self.write_count(e.child_inventories.len()));
        for i in e.child_inventories() {
            try!(self.write_inventory(&i));
        }

        Ok(())
    }

    fn write_structure(&mut self, s: &ObjectRef<Structure>) -> Result<()> {
        try!(self.write_object_header(s));

        // Body
        try!(self.write_v3(s.pos));
        try!(self.write_template_id(s.world().data(), s.template));

        // Children
        try!(self.write_count(s.child_inventories.len()));
        for i in s.child_inventories() {
            try!(self.write_inventory(&i));
        }

        Ok(())
    }

    fn write_inventory(&mut self, i: &ObjectRef<Inventory>) -> Result<()> {
        try!(self.write_object_header(i));

        // Body
        try!(self.write_count(i.contents.len()));
        for (name, count) in i.contents.iter() {
            try!(self.writer.write_u8(*count));
            try!(self.writer.write_u8(unwrap!(name.len().to_u8())));
            try!(self.writer.write_le_u16(0));
            try!(self.write_str_bytes(&**name));
        }

        Ok(())
    }

    fn reset(&mut self) {
        self.id_map.clear();
        self.next_id = 0;
        self.objects_written.clear();
        self.seen_templates.clear();
    }

    pub fn save_client(&mut self, c: &ObjectRef<Client>) -> Result<()> {
        self.reset();
        try!(self.write_header());
        try!(self.write_client(c));
        try!(self.handle_missing());
        self.reset();

        Ok(())
    }

    pub fn save_terrain_chunk(&mut self, t: &ObjectRef<TerrainChunk>) -> Result<()> {
        self.reset();
        try!(self.write_header());
        try!(self.write_terrain_chunk(t));
        try!(self.handle_missing());
        self.reset();

        Ok(())
    }

    fn handle_missing(&mut self) -> Result<()> {
        for &aid in self.id_map.keys() {
            if self.objects_written.contains(&aid) {
                continue;
            }

            fail!("reference to object not in tree");
        }
        Ok(())
    }
}


pub struct SaveReader<R: Reader> {
    reader: R,

    id_map: HashMap<SaveId, AnyId>,
    template_map: HashMap<TemplateId, TemplateId>,
    created_objs: HashSet<AnyId>,
    inited_objs: HashSet<AnyId>,
}

impl<R: Reader> SaveReader<R> {
    pub fn new(reader: R) -> SaveReader<R> {
        SaveReader {
            reader: reader,
            id_map: HashMap::new(),
            template_map: HashMap::new(),
            created_objs: HashSet::new(),
            inited_objs: HashSet::new(),
        }
    }

    fn read_header(&mut self) -> Result<()> {
        let version = try!(self.reader.read_le_u32());
        if version != CURRENT_VERSION {
            fail!("file version does not match current version");
        }
        Ok(())
    }

    fn read_id_helper<T: ReadId>(&mut self, w: &mut World, save_id: SaveId) -> Result<T> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        match self.id_map.entry(save_id) {
            Occupied(e) => ReadId::from_any_id(*e.get()),
            Vacant(e) => {
                let id = <T as ReadId>::fabricate(w);
                self.created_objs.insert(id.to_any_id());
                e.insert(id.to_any_id());
                Ok(id)
            },
        }
    }

    fn read_id<T: ReadId>(&mut self, w: &mut World) -> Result<T> {
        let save_id = try!(self.reader.read_le_u32());
        self.read_id_helper(w, save_id)
    }

    fn read_opt_id<T: ReadId>(&mut self, w: &mut World) -> Result<Option<T>> {
        let save_id = try!(self.reader.read_le_u32());
        if save_id == -1 as SaveId {
            Ok(None)
        } else {
            let id = try!(self.read_id_helper(w, save_id));
            Ok(Some(id))
        }
    }

    fn read_object_header<T: ReadId>(&mut self, w: &mut World) -> Result<(T, StableId)> {
        let id: T = try!(self.read_id(w));
        let stable_id = try!(self.reader.read_le_u64());
        self.inited_objs.insert(id.to_any_id());
        Ok((id, stable_id))
    }

    fn read_count(&mut self) -> Result<usize> {
        let count = try!(self.reader.read_le_u32());
        Ok(unwrap!(count.to_uint()))
    }

    fn read_v2(&mut self) -> Result<V2> {
        let x = try!(self.reader.read_le_i32());
        let y = try!(self.reader.read_le_i32());
        Ok(V2::new(x, y))
    }

    fn read_v3(&mut self) -> Result<V3> {
        let x = try!(self.reader.read_le_i32());
        let y = try!(self.reader.read_le_i32());
        let z = try!(self.reader.read_le_i32());
        Ok(V3::new(x, y, z))
    }

    fn read_str(&mut self, len: usize) -> Result<String> {
        let padding = (4 - (len % 4)) % 4;
        let mut vec = try!(self.reader.read_exact(len + padding));
        vec.truncate(len);
        match String::from_utf8(vec) {
            Ok(s) => Ok(s),
            Err(_) => fail!("utf8 encoding error"),
        }
    }

    fn read_template_id(&mut self, data: &Data) -> Result<TemplateId> {
        let old_id = try!(self.reader.read_le_u32());
        match self.template_map.get(&old_id) {
            Some(&new_id) => return Ok(new_id),
            None => {},
        }

        // First time seeing this ID.  Read the definition.
        let x = try!(self.reader.read_u8());
        let y = try!(self.reader.read_u8());
        let z = try!(self.reader.read_u8());
        let size = V3::new(unwrap!(x.to_i32()),
                           unwrap!(y.to_i32()),
                           unwrap!(z.to_i32()));
        let name_len = try!(self.reader.read_u8());
        let name = try!(self.read_str(unwrap!(name_len.to_uint())));

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

        let pawn_id = try!(self.read_opt_id(w));

        {
            let c = &mut w.clients[cid];
            c.stable_id = stable_id;

            let name_len = try!(self.read_count());
            let name = try!(self.read_str(name_len));

            c.name = name;
            c.pawn = pawn_id;
        }
        // At this point all Client invariants hold, except that c.pawn is not yet attached to the
        // client.

        let child_entity_count = try!(self.read_count());
        for _ in range(0, child_entity_count) {
            let eid = try!(self.read_entity(w));
            try!(ops::entity_attach(w, eid, EntityAttachment::Client(cid)));
        }

        let child_inventory_count = try!(self.read_count());
        for _ in range(0, child_inventory_count) {
            let _iid = try!(self.read_inventory(w));
            // TODO: implement inventory_attach
            //try!(ops::inventory_attach(self.world, iid, InventoryAttachment::Client(cid)));
        }

        Ok(cid)
    }

    fn read_terrain_chunk(&mut self, w: &mut World) -> Result<V2> {
        let save_id = try!(self.reader.read_le_u32());
        let chunk_pos = try!(self.read_v2());
        self.id_map.insert(save_id, AnyId::TerrainChunk(chunk_pos));

        let mut blocks = Box::new([0; CHUNK_TOTAL]);
        {
            let byte_len = blocks.len() * mem::size_of::<BlockId>();
            let byte_array = unsafe {
                mem::transmute(raw::Slice {
                    data: blocks.as_ptr() as *const u8,
                    len: byte_len,
                })
            };
            try!(self.reader.read_at_least(CHUNK_TOTAL, byte_array));
        }

        let mut block_map = HashMap::new();
        let block_id_count = try!(self.read_count());
        let block_data = &w.data().block_data;
        for _ in range(0, block_id_count) {
            let old_id = try!(self.reader.read_le_u16());
            let shape = try!(self.reader.read_u8());
            let name_len = try!(self.reader.read_u8());
            let name = try!(self.read_str(unwrap!(name_len.to_uint())));
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

        let child_structure_count = try!(self.read_count());
        for _ in range(0, child_structure_count) {
            let sid = try!(self.read_structure(w));
            try!(ops::structure_attach(w, sid, StructureAttachment::Chunk));
        }

        Ok(chunk_pos)
    }

    fn read_entity(&mut self, w: &mut World) -> Result<EntityId> {
        let (eid, stable_id) = try!(self.read_object_header(w));

        {
            let e = &mut w.entities[eid];
            e.stable_id = stable_id;

            e.motion.start_pos = try!(self.read_v3());
            e.motion.end_pos = try!(self.read_v3());
            e.motion.start_time = try!(self.reader.read_le_i64());
            e.motion.duration = try!(self.reader.read_le_u16());

            e.anim = try!(self.reader.read_le_u16());
            e.facing = try!(self.read_v3());
            e.target_velocity = try!(self.read_v3());
        }

        let child_inventory_count = try!(self.read_count());
        for _ in range(0, child_inventory_count) {
            let _iid = try!(self.read_inventory(w));
            // TODO: implement inventory_attach
            //try!(ops::inventory_attach(self.world, iid, InventoryAttachment::Entity(cid)));
        }

        Ok(eid)
    }

    fn read_structure(&mut self, w: &mut World) -> Result<StructureId> {
        let (sid, stable_id) = try!(self.read_object_header(w));
        let data = w.data();

        {
            let s = &mut w.structures[sid];
            s.stable_id = stable_id;

            s.pos = try!(self.read_v3());
            s.template = try!(self.read_template_id(data));
        }

        try!(ops::structure_post_init(w, sid));

        let child_inventory_count = try!(self.read_count());
        for _ in range(0, child_inventory_count) {
            let _iid = try!(self.read_inventory(w));
            // TODO: implement inventory_attach
            //try!(ops::inventory_attach(self.world, iid, InventoryAttachment::Structure(cid)));
        }

        Ok(sid)
    }

    fn read_inventory(&mut self, w: &mut World) -> Result<InventoryId> {
        let (iid, stable_id) = try!(self.read_object_header(w));

        {
            let i = &mut w.inventories[iid];
            i.stable_id = stable_id;

            let contents_count = try!(self.read_count());
            for _ in range(0, contents_count) {
                let count = try!(self.reader.read_u8());
                let name_len = try!(self.reader.read_u8());
                try!(self.reader.read_le_u16());
                let name = try!(self.read_str(unwrap!(name_len.to_uint())));
                i.contents.insert(name, count);
            }
        }

        Ok(iid)
    }

    fn reset(&mut self) {
        self.id_map.clear();
        self.template_map.clear();
        self.created_objs.clear();
        self.inited_objs.clear();
    }

    fn load_wrapper<T, F>(&mut self, w: &mut World, f: F) -> Result<T>
            where F: FnOnce(&mut SaveReader<R>, &mut World) -> Result<T> {
        self.reset();
        try!(self.read_header());
        let result = f(self, w);
        let result = result.and_then(|x| { try!(self.check_objs()); Ok(x) });

        if result.is_err() {
            self.cleanup(w);
        }

        self.reset();
        result
    }

    pub fn load_client(&mut self, w: &mut World) -> Result<ClientId> {
        self.load_wrapper(w, |self_, w| self_.read_client(w))
    }

    pub fn load_terrain_chunk(&mut self, w: &mut World) -> Result<V2> {
        self.load_wrapper(w, |self_, w| self_.read_terrain_chunk(w))
    }

    fn check_objs(&mut self) -> Result<()> {
        match self.created_objs.difference(&self.inited_objs).next() {
            None => Ok(()),
            Some(_) => fail!("object was referenced but not defined"),
        }
    }

    fn cleanup(&mut self, w: &mut World) {
        fn unwrap_warn<T, E: error::Error>(r: result::Result<T, E>) {
            match r {
                Ok(_) => {},
                Err(e) => warn!("error occurred during cleanup: {}",
                                error::Error::description(&e)),
            }
        }

        for &aid in self.created_objs.iter() {
            match aid {
                AnyId::Client(cid) => {
                    w.clients.remove(cid);
                },
                AnyId::TerrainChunk(pos) => {
                    w.terrain_chunks.remove(&pos);
                },
                AnyId::Entity(eid) => {
                    w.entities.remove(eid);
                },
                AnyId::Structure(sid) => {
                    unwrap_warn(ops::structure_pre_fini(w, sid));
                    w.structures.remove(sid);
                },
                AnyId::Inventory(iid) => {
                    w.inventories.remove(iid);
                },
            }
        }
    }
}
