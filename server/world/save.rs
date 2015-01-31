use std::collections::{HashMap, HashSet};
use std::error;
use std::io::{self, IoResult};
use std::mem;
use std::num::ToPrimitive;
use std::raw;
use std::result;

use physics::v3::{Vn, V3, V2, scalar};

use data::Data;
use types::*;
use util::IntrusiveStableId;
use util::StrError;
use world::{World, Client, TerrainChunk, Entity, Structure, Inventory};
use world::StructureAttachment;
use world::object::*;

type SaveId = u32;

#[derive(Copy, PartialEq, Eq, Show, Hash)]
enum AnyId {
    Client(ClientId),
    TerrainChunk(V2),
    Entity(EntityId),
    Structure(StructureId),
    Inventory(InventoryId),

    /*
    StableClient(Stable<ClientId>),
    StableEntity(Stable<EntityId>),
    StableStructure(Stable<StructureId>),
    StableInventory(Stable<InventoryId>),
    */
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


#[derive(Show)]
enum Error {
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

type Result<T> = result::Result<T, Error>;


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

    fn take_id(&mut self) -> SaveId {
        let id = self.next_id;
        self.next_id += 1;
        id
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
            self.write_str_bytes(&*template.name);
        }
        Ok(())
    }


    fn write_client(&mut self, c: &ObjectRef<Client>) -> Result<()> {
        try!(self.write_object_header(c));

        // Body
        try!(self.write_opt_id(c.pawn));

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
            self.write_str_bytes(name);
        }

        // Children
        // TODO: We have to do our own filtering here because ClientRef::child_structures is not
        // implemented.
        let child_structures = t.world().chunk_structures(t.id())
                                .filter(|s| t.bounds().contains(s.pos) &&
                                            s.attachment == StructureAttachment::Chunk)
                                .collect::<Vec<_>>();
        try!(self.write_count(child_structures.len()));
        for s in child_structures.into_iter() {
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
