use std::collections::{HashMap, HashSet};

use physics::CHUNK_SIZE;
use types::*;
use util::{multimap_insert, multimap_remove};
use util::stable_id_map::NO_STABLE_ID;

use world::{Structure, StructureAttachment};
use world::{Fragment, Hooks};
use world::ops::{self, OpResult};


pub fn create<'d, F>(f: &mut F,
                     pos: V3,
                     tid: TemplateId) -> OpResult<StructureId>
        where F: Fragment<'d> {
    let t = unwrap!(f.world().data.structure_templates.get_template(tid));
    let bounds = Region::new(pos, pos + t.size);

    if !f.with_hooks(|h| h.check_structure_placement(t, pos)) {
        fail!("structure placement blocked by terrain or other structure");
    }

    let s = Structure {
        pos: pos,
        template: tid,

        stable_id: NO_STABLE_ID,
        attachment: StructureAttachment::World,
        child_inventories: HashSet::new(),
    };

    let sid = unwrap!(f.world_mut().structures.insert(s));
    add_to_lookup(&mut f.world_mut().structures_by_chunk, sid, bounds);
    invalidate_region(f, bounds);
    f.with_hooks(|h| h.on_structure_create(sid));
    Ok(sid)
}

pub fn create_unchecked<'d, F>(f: &mut F) -> StructureId
        where F: Fragment<'d> {
    let sid = f.world_mut().structures.insert(Structure {
        pos: scalar(0),
        template: 0,

        stable_id: NO_STABLE_ID,
        attachment: StructureAttachment::World,
        child_inventories: HashSet::new(),
    }).unwrap();     // Shouldn't fail when stable_id == NO_STABLE_ID
    sid
}

pub fn post_init<'d, F>(f: &mut F,
                        sid: StructureId) -> OpResult<()>
        where F: Fragment<'d> {
    let bounds = {
        let s = unwrap!(f.world().structures.get(sid));
        let t = unwrap!(f.world().data.structure_templates.get_template(s.template));

        Region::new(s.pos, s.pos + t.size)
    };

    add_to_lookup(&mut f.world_mut().structures_by_chunk, sid, bounds);
    invalidate_region(f, bounds);
    Ok(())
}

pub fn pre_fini<'d, F>(f: &mut F,
                       sid: StructureId) -> OpResult<()>
        where F: Fragment<'d> {
    let bounds = {
        let s = unwrap!(f.world().structures.get(sid));
        let t = unwrap!(f.world().data.structure_templates.get_template(s.template));

        Region::new(s.pos, s.pos + t.size)
    };

    remove_from_lookup(&mut f.world_mut().structures_by_chunk, sid, bounds);
    invalidate_region(f, bounds);
    Ok(())
}

pub fn destroy<'d, F>(f: &mut F,
                      sid: StructureId) -> OpResult<()>
        where F: Fragment<'d> {
    use world::StructureAttachment::*;
    let s = unwrap!(f.world_mut().structures.remove(sid));

    let t = f.world().data.structure_templates.template(s.template);
    let bounds = Region::new(s.pos, s.pos + t.size);
    remove_from_lookup(&mut f.world_mut().structures_by_chunk, sid, bounds);
    invalidate_region(f, bounds);

    match s.attachment {
        World => {},
        Chunk => {
            let chunk_pos = s.pos.reduce().div_floor(scalar(CHUNK_SIZE));
            // Chunk may not be loaded, since destruction proceeds top-down.
            f.world_mut().terrain_chunks.get_mut(&chunk_pos)
             .map(|t| t.child_structures.remove(&sid));
        },
    }

    for &iid in s.child_inventories.iter() {
        ops::inventory::destroy(f, iid).unwrap();
    }

    f.with_hooks(|h| h.on_structure_destroy(sid, bounds));
    Ok(())
}

pub fn attach<'d, F>(f: &mut F,
                     sid: StructureId,
                     new_attach: StructureAttachment) -> OpResult<StructureAttachment>
        where F: Fragment<'d> {
    use world::StructureAttachment::*;

    let w = f.world_mut();
    let s = unwrap!(w.structures.get_mut(sid));
    let old_attach = s.attachment;

    if new_attach == old_attach {
        return Ok(new_attach);
    }

    let chunk_pos = s.pos().reduce().div_floor(scalar(CHUNK_SIZE));

    match new_attach {
        World => {},
        Chunk => {
            let t = unwrap!(w.terrain_chunks.get_mut(&chunk_pos),
                            "can't attach structure to unloaded chunk");
            // No more checks beyond this point.
            t.child_structures.insert(sid);
        },
    }

    match old_attach {
        World => {},
        Chunk => {
            // If we're detaching from Chunk, we know the containing chunk is loaded because `c` is
            // loaded and has attachment Chunk.
            w.terrain_chunks[chunk_pos].child_structures.remove(&sid);
        },
    }

    s.attachment = new_attach;
    Ok(old_attach)
}

pub fn replace<'d, F>(f: &mut F,
                      sid: StructureId,
                      new_tid: TemplateId) -> OpResult<()>
        where F: Fragment<'d> {
    let bounds = {
        let w = f.world_mut();
        let s = unwrap!(w.structures.get_mut(sid));

        let old_t = unwrap!(w.data.structure_templates.get_template(s.template));
        let new_t = unwrap!(w.data.structure_templates.get_template(new_tid));

        if old_t.size != new_t.size ||
           old_t.shape != new_t.shape ||
           old_t.layer != new_t.layer {
            fail!("replacement structure template differs in shape");
        }

        s.template = new_tid;

        Region::new(s.pos, s.pos + old_t.size)
    };

    invalidate_region(f, bounds);
    f.with_hooks(|h| h.on_structure_replace(sid, bounds));
    Ok(())
}

fn add_to_lookup(lookup: &mut HashMap<V2, HashSet<StructureId>>,
                 sid: StructureId,
                 bounds: Region) {
    let chunk_bounds = bounds.reduce().div_round_signed(CHUNK_SIZE);
    for chunk_pos in chunk_bounds.points() {
        multimap_insert(lookup, chunk_pos, sid);
    }
}

fn remove_from_lookup(lookup: &mut HashMap<V2, HashSet<StructureId>>,
                      sid: StructureId,
                      bounds: Region) {
    let chunk_bounds = bounds.reduce().div_round_signed(CHUNK_SIZE);
    for chunk_pos in chunk_bounds.points() {
        multimap_remove(lookup, chunk_pos, sid);
    }
}

fn invalidate_region<'d, F>(f: &mut F,
                            bounds: Region)
        where F: Fragment<'d> {
    let chunk_bounds = bounds.reduce().div_round_signed(CHUNK_SIZE);
    for chunk_pos in chunk_bounds.points() {
        f.with_hooks(|h| h.on_chunk_invalidate(chunk_pos));
    }
}
