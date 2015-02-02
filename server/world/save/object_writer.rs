use std::collections::HashSet;
use std::io;
use std::mem;
use std::num::ToPrimitive;
use std::raw;

use data::Data;
use types::*;
use util::IntrusiveStableId;
use world::{Client, TerrainChunk, Entity, Structure, Inventory};
use world::object::*;

use super::Result;
use super::{AnyId, ToAnyId};
use super::writer::{Writer, WriterWrapper};
use super::CURRENT_VERSION;


pub struct ObjectWriter<W: io::Writer, H: WriteHooks> {
    w: WriterWrapper<W>,
    hooks: H,
    objects_written: HashSet<AnyId>,
    seen_templates: HashSet<TemplateId>,
}

#[allow(unused_variables)]
pub trait WriteHooks {
    fn post_write_client<W: Writer>(&mut self,
                                    writer: &mut W,
                                    c: &ObjectRef<Client>) -> Result<()> { Ok(()) }
    fn post_write_terrain_chunk<W: Writer>(&mut self,
                                           writer: &mut W,
                                           t: &ObjectRef<TerrainChunk>) -> Result<()> { Ok(()) }
    fn post_write_entity<W: Writer>(&mut self,
                                    writer: &mut W,
                                    e: &ObjectRef<Entity>) -> Result<()> { Ok(()) }
    fn post_write_structure<W: Writer>(&mut self,
                                       writer: &mut W,
                                       s: &ObjectRef<Structure>) -> Result<()> { Ok(()) }
    fn post_write_inventory<W: Writer>(&mut self,
                                       writer: &mut W,
                                       i: &ObjectRef<Inventory>) -> Result<()> { Ok(()) }
}

impl<W: io::Writer, H: WriteHooks> ObjectWriter<W, H> {
    pub fn new(writer: W, hooks: H) -> ObjectWriter<W, H> {
        ObjectWriter {
            w: WriterWrapper::new(writer),
            hooks: hooks,
            objects_written: HashSet::new(),
            seen_templates: HashSet::new(),
        }
    }

    fn write_file_header(&mut self) -> Result<()> {
        self.w.write(CURRENT_VERSION)
    }

    fn write_object_header<O>(&mut self, o: &ObjectRef<O>) -> Result<()>
            where O: Object+IntrusiveStableId,
                  <O as Object>::Id: ToAnyId {
        // NB: write_terrain_chunk contains a specialized copy of this code that can deal with
        // TerrainChunk's idiosyncrasies.
        try!(self.w.write_id(o.id()));
        try!(self.w.write(o.get_stable_id()));
        self.objects_written.insert(o.id().to_any_id());
        Ok(())
    }

    fn write_template_id(&mut self, data: &Data, template_id: TemplateId) -> Result<()> {
        try!(self.w.write(template_id));
        if !self.seen_templates.contains(&template_id) {
            self.seen_templates.insert(template_id);

            let template = data.object_templates.template(template_id);
            try!(self.w.write((unwrap!(template.size.x.to_u8()),
                               unwrap!(template.size.y.to_u8()),
                               unwrap!(template.size.z.to_u8()),
                               unwrap!(template.name.len().to_u8()))));
            try!(self.w.write_str_bytes(&*template.name));
        }
        Ok(())
    }


    fn write_client(&mut self, c: &ObjectRef<Client>) -> Result<()> {
        try!(self.write_object_header(c));

        // Body
        try!(self.w.write_opt_id(c.pawn));
        try!(self.w.write_str(c.name()));

        try!(self.hooks.post_write_client(&mut self.w, c));

        // Children
        try!(self.w.write_count(c.child_entities.len()));
        for e in c.child_entities() {
            try!(self.write_entity(&e));
        }

        try!(self.w.write_count(c.child_inventories.len()));
        for i in c.child_inventories() {
            try!(self.write_inventory(&i));
        }

        Ok(())
    }

    fn write_terrain_chunk(&mut self, t: &ObjectRef<TerrainChunk>) -> Result<()> {
        // Write a custom object header.  Assign it a SaveId like normal, but use the chunk
        // position as a stable ID.
        let aid = AnyId::TerrainChunk(t.id());
        try!(self.w.write_id(aid));
        try!(self.w.write(t.id()));
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
        try!(self.w.write_bytes(byte_array));

        // Body - lookup table
        let blocks_seen = t.blocks.iter().map(|&x| x).collect::<HashSet<_>>();
        try!(self.w.write_count(blocks_seen.len()));
        let block_data = &t.world().data().block_data;
        for b in blocks_seen.into_iter() {
            let shape = block_data.shape(b);
            let name = block_data.name(b);
            try!(self.w.write((b,
                               shape as u8,
                               unwrap!(name.len().to_u8()))));
            try!(self.w.write_str_bytes(name));
        }

        try!(self.hooks.post_write_terrain_chunk(&mut self.w, t));

        // Children
        try!(self.w.write_count(t.child_structures.len()));
        for s in t.child_structures() {
            try!(self.write_structure(&s));
        }

        Ok(())
    }

    fn write_entity(&mut self, e: &ObjectRef<Entity>) -> Result<()> {
        try!(self.write_object_header(e));

        // Body
        let m = &e.motion;
        try!(self.w.write((m.start_pos,
                           m.end_pos,
                           m.start_time,
                           m.duration, e.anim,  // u16 * 2
                           e.facing,
                           e.target_velocity)));

        try!(self.hooks.post_write_entity(&mut self.w, e));

        // Children
        try!(self.w.write_count(e.child_inventories.len()));
        for i in e.child_inventories() {
            try!(self.write_inventory(&i));
        }

        Ok(())
    }

    fn write_structure(&mut self, s: &ObjectRef<Structure>) -> Result<()> {
        try!(self.write_object_header(s));

        // Body
        try!(self.w.write(s.pos));
        try!(self.write_template_id(s.world().data(), s.template));

        try!(self.hooks.post_write_structure(&mut self.w, s));

        // Children
        try!(self.w.write_count(s.child_inventories.len()));
        for i in s.child_inventories() {
            try!(self.write_inventory(&i));
        }

        Ok(())
    }

    fn write_inventory(&mut self, i: &ObjectRef<Inventory>) -> Result<()> {
        try!(self.write_object_header(i));

        // Body
        try!(self.w.write_count(i.contents.len()));
        for (name, count) in i.contents.iter() {
            try!(self.w.write((*count,
                               unwrap!(name.len().to_u8()))));
            try!(self.w.write_str_bytes(&**name));
        }

        try!(self.hooks.post_write_inventory(&mut self.w, i));

        Ok(())
    }

    fn save_object<F>(&mut self, f: F) -> Result<()>
            where F: FnOnce(&mut ObjectWriter<W, H>) -> Result<()> {
        try!(self.write_file_header());
        try!(f(self));
        try!(self.handle_missing());
        Ok(())
    }

    pub fn save_client(&mut self, c: &ObjectRef<Client>) -> Result<()> {
        self.save_object(|sw| sw.write_client(c))
    }

    pub fn save_terrain_chunk(&mut self, t: &ObjectRef<TerrainChunk>) -> Result<()> {
        self.save_object(|sw| sw.write_terrain_chunk(t))
    }

    fn handle_missing(&mut self) -> Result<()> {
        for &aid in self.w.id_map().keys() {
            if self.objects_written.contains(&aid) {
                continue;
            }

            fail!("reference to object not in tree");
        }
        Ok(())
    }
}


pub struct NoWriteHooks;

impl WriteHooks for NoWriteHooks { }
