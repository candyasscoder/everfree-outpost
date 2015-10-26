use std::collections::HashSet;
use std::io;
use std::mem;
use std::slice;

use libphysics::CHUNK_SIZE;
use types::*;

use data::Data;
use util::Convert;
use util::IntrusiveStableId;
use world::{World, Client, Entity, Inventory, Plane, TerrainChunk, Structure};
use world::Item;
use world::object::*;

use super::Result;
use super::{AnyId, ToAnyId};
use super::writer::{Writer, WriterWrapper};
use super::CURRENT_VERSION;


pub struct ObjectWriter<W: io::Write, H: WriteHooks> {
    w: WriterWrapper<W>,
    hooks: H,
    objects_written: HashSet<AnyId>,
    seen_templates: HashSet<TemplateId>,
    seen_items: HashSet<ItemId>,
}

#[allow(unused_variables)]
pub trait WriteHooks {
    fn post_write_world<W: Writer>(&mut self,
                                   writer: &mut W,
                                   w: &World) -> Result<()> { Ok(()) }
    fn post_write_client<W: Writer>(&mut self,
                                    writer: &mut W,
                                    c: &ObjectRef<Client>) -> Result<()> { Ok(()) }
    fn post_write_entity<W: Writer>(&mut self,
                                    writer: &mut W,
                                    e: &ObjectRef<Entity>) -> Result<()> { Ok(()) }
    fn post_write_inventory<W: Writer>(&mut self,
                                       writer: &mut W,
                                       i: &ObjectRef<Inventory>) -> Result<()> { Ok(()) }
    fn post_write_plane<W: Writer>(&mut self,
                                   writer: &mut W,
                                   t: &ObjectRef<Plane>) -> Result<()> { Ok(()) }
    fn post_write_terrain_chunk<W: Writer>(&mut self,
                                           writer: &mut W,
                                           t: &ObjectRef<TerrainChunk>) -> Result<()> { Ok(()) }
    fn post_write_structure<W: Writer>(&mut self,
                                       writer: &mut W,
                                       s: &ObjectRef<Structure>) -> Result<()> { Ok(()) }
}

impl<W: io::Write, H: WriteHooks> ObjectWriter<W, H> {
    pub fn new(writer: W, hooks: H) -> ObjectWriter<W, H> {
        ObjectWriter {
            w: WriterWrapper::new(writer),
            hooks: hooks,
            objects_written: HashSet::new(),
            seen_templates: HashSet::new(),
            seen_items: HashSet::new(),
        }
    }

    fn write_file_header(&mut self) -> Result<()> {
        self.w.write(CURRENT_VERSION)
    }

    fn write_object_header<O>(&mut self, o: &ObjectRef<O>) -> Result<()>
            where O: Object+IntrusiveStableId,
                  <O as Object>::Id: ToAnyId {
        try!(self.w.write_id(o.id()));
        try!(self.w.write(o.get_stable_id()));
        self.objects_written.insert(o.id().to_any_id());
        Ok(())
    }

    fn write_template_id(&mut self, data: &Data, template_id: TemplateId) -> Result<()> {
        try!(self.w.write(template_id));
        if !self.seen_templates.contains(&template_id) {
            self.seen_templates.insert(template_id);

            let template = data.structure_templates.template(template_id);
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
        // Don't write `name`.  It will be reconstructed from metadata.

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

    fn write_entity(&mut self, e: &ObjectRef<Entity>) -> Result<()> {
        try!(self.write_object_header(e));

        // Body
        let m = &e.motion;
        try!(self.w.write((e.stable_plane.unwrap(),
                           m.start_pos,
                           m.end_pos,
                           m.start_time,
                           m.duration, e.anim,  // u16 * 2
                           e.facing,
                           e.target_velocity,
                           e.appearance)));

        try!(self.hooks.post_write_entity(&mut self.w, e));

        // Children
        try!(self.w.write_count(e.child_inventories.len()));
        for i in e.child_inventories() {
            try!(self.write_inventory(&i));
        }

        Ok(())
    }

    fn write_inventory(&mut self, i: &ObjectRef<Inventory>) -> Result<()> {
        try!(self.write_object_header(i));
        let data = i.world().data();

        // Body
        // Count goes first so it can be distinguished from item names and slots.
        try!(self.w.write_count(i.contents.len()));
        // New item names.  Note that the number of these is not known in advance.
        for slot in i.contents.iter() {
            let item_id =
                match *slot {
                    Item::Empty => continue,
                    Item::Bulk(_, item_id) => item_id,
                    Item::Special(_, item_id) => item_id,
                };

            if !self.seen_items.contains(&item_id) {
                self.seen_items.insert(item_id);
                let name = data.item_data.name(item_id);
                // Same format as inventory contents, but with special tag 255
                try!(self.w.write((255_u8, unwrap!(name.len().to_u8()), item_id)));
                try!(self.w.write_str_bytes(name));
            }
        }
        // Actual inventory contents
        for slot in i.contents.iter() {
            let val = 
                match *slot {
                    Item::Empty => (0, 0, 0),
                    Item::Bulk(count, item_id) => (1, count, item_id),
                    Item::Special(script_id, item_id) => (2, script_id, item_id),
                };
            try!(self.w.write(val));
        }

        try!(self.hooks.post_write_inventory(&mut self.w, i));

        Ok(())
    }

    fn write_plane(&mut self, p: &ObjectRef<Plane>) -> Result<()> {
        try!(self.write_object_header(p));

        // Body
        try!(self.w.write_str(p.name()));

        try!(self.w.write_count(p.saved_chunks.len()));
        for (&cpos, &stable_tcid) in p.saved_chunks.iter() {
            try!(self.w.write((cpos, stable_tcid.unwrap())));
        }

        try!(self.hooks.post_write_plane(&mut self.w, p));
        Ok(())
    }

    fn write_terrain_chunk(&mut self, t: &ObjectRef<TerrainChunk>) -> Result<()> {
        try!(self.write_object_header(t));

        // Don't write `plane` or `cpos`.  These will be reconstructed from metadata.

        try!(self.w.write(t.flags.bits()));

        // Body - block data
        let len = t.blocks.len();
        let byte_len = len * mem::size_of::<BlockId>();
        let byte_array = unsafe {
            slice::from_raw_parts(t.blocks.as_ptr() as *const u8, byte_len)
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
        let base = t.cpos.extend(0) * scalar(CHUNK_SIZE);
        for s in t.child_structures() {
            try!(self.write_structure(&s, base));
        }

        Ok(())
    }

    fn write_structure(&mut self, s: &ObjectRef<Structure>, base: V3) -> Result<()> {
        try!(self.write_object_header(s));

        // Body
        // Don't write `plane`.  It will be reconstructed from metadata.
        // Write only the offset of `pos` from `base`.
        try!(self.w.write(s.pos - base));
        try!(self.write_template_id(s.world().data(), s.template));

        try!(self.w.write(s.flags.bits()));

        try!(self.hooks.post_write_structure(&mut self.w, s));

        // Children
        try!(self.w.write_count(s.child_inventories.len()));
        for i in s.child_inventories() {
            try!(self.write_inventory(&i));
        }

        Ok(())
    }

    fn write_world(&mut self, w: &World) -> Result<()> {
        use world::{EntityAttachment, InventoryAttachment};

        try!(self.w.write(w.clients.next_id()));
        try!(self.w.write(w.entities.next_id()));
        try!(self.w.write(w.inventories.next_id()));
        try!(self.w.write(w.planes.next_id()));
        try!(self.w.write(w.terrain_chunks.next_id()));
        try!(self.w.write(w.structures.next_id()));

        try!(self.hooks.post_write_world(&mut self.w, w));

        {
            let es = w.entities()
                      .filter(|e| e.attachment() == EntityAttachment::World)
                      .collect::<Vec<_>>();
            try!(self.w.write_count(es.len()));
            for e in es.into_iter() {
                try!(self.write_entity(&e));
            }
        }

        {
            let is = w.inventories()
                      .filter(|i| i.attachment() == InventoryAttachment::World)
                      .collect::<Vec<_>>();
            try!(self.w.write_count(is.len()));
            for i in is.into_iter() {
                try!(self.write_inventory(&i));
            }
        }

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

    pub fn save_plane(&mut self, p: &ObjectRef<Plane>) -> Result<()> {
        self.save_object(|sw| sw.write_plane(p))
    }

    pub fn save_terrain_chunk(&mut self, t: &ObjectRef<TerrainChunk>) -> Result<()> {
        self.save_object(|sw| sw.write_terrain_chunk(t))
    }

    pub fn save_world(&mut self, w: &World) -> Result<()> {
        self.save_object(|sw| sw.write_world(w))
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
