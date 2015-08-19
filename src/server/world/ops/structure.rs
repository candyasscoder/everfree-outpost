use std::collections::{HashMap, HashSet};

use libphysics::CHUNK_SIZE;
use types::*;
use util::{multimap_insert, multimap_remove};
use util::stable_id_map::NO_STABLE_ID;

use world::{Structure, StructureAttachment, StructureFlags};
use world::{Fragment, Hooks};
use world::ops::{self, OpResult};


pub fn create<'d, F>(f: &mut F,
                     pid: PlaneId,
                     pos: V3,
                     tid: TemplateId) -> OpResult<StructureId>
        where F: Fragment<'d> {
    let t = unwrap!(f.world().data.structure_templates.get_template(tid));
    let bounds = Region::new(pos, pos + t.size);

    if bounds.min.z < 0 || bounds.max.z > CHUNK_SIZE {
        fail!("structure placement blocked by map bounds");
    }

    if !f.with_hooks(|h| h.check_structure_placement(t, pid, pos)) {
        fail!("structure placement blocked by terrain or other structure");
    }

    let s = Structure {
        plane: pid,
        pos: pos,
        template: tid,

        stable_id: NO_STABLE_ID,
        flags: StructureFlags::empty(),
        attachment: StructureAttachment::Plane,
        child_inventories: HashSet::new(),
    };

    let sid = unwrap!(f.world_mut().structures.insert(s));
    add_to_lookup(&mut f.world_mut().structures_by_chunk, sid, pid, bounds);
    f.with_hooks(|h| h.on_structure_create(sid));
    Ok(sid)
}

pub fn create_unchecked<'d, F>(f: &mut F) -> StructureId
        where F: Fragment<'d> {
    let sid = f.world_mut().structures.insert(Structure {
        plane: PlaneId(0),
        pos: scalar(0),
        template: 0,

        stable_id: NO_STABLE_ID,
        flags: StructureFlags::empty(),
        attachment: StructureAttachment::Plane,
        child_inventories: HashSet::new(),
    }).unwrap();     // Shouldn't fail when stable_id == NO_STABLE_ID
    sid
}

pub fn post_init<'d, F>(f: &mut F,
                        sid: StructureId) -> OpResult<()>
        where F: Fragment<'d> {
    let (pid, bounds) = {
        let s = unwrap!(f.world().structures.get(sid));
        let t = unwrap!(f.world().data.structure_templates.get_template(s.template));

        (s.plane, Region::new(s.pos, s.pos + t.size))
    };

    add_to_lookup(&mut f.world_mut().structures_by_chunk, sid, pid, bounds);
    Ok(())
}

pub fn pre_fini<'d, F>(f: &mut F,
                       sid: StructureId) -> OpResult<()>
        where F: Fragment<'d> {
    let (pid, bounds) = {
        let s = unwrap!(f.world().structures.get(sid));
        let t = unwrap!(f.world().data.structure_templates.get_template(s.template));

        (s.plane, Region::new(s.pos, s.pos + t.size))
    };

    remove_from_lookup(&mut f.world_mut().structures_by_chunk, sid, pid, bounds);
    Ok(())
}

pub fn destroy<'d, F>(f: &mut F,
                      sid: StructureId) -> OpResult<()>
        where F: Fragment<'d> {
    use world::StructureAttachment::*;
    let s = unwrap!(f.world_mut().structures.remove(sid));

    let t = f.world().data.structure_templates.template(s.template);
    let bounds = Region::new(s.pos, s.pos + t.size);
    remove_from_lookup(&mut f.world_mut().structures_by_chunk, sid, s.plane, bounds);

    match s.attachment {
        // TODO: proper support for Plane attachment
        Plane => {},
        Chunk => {
            let w = f.world_mut();
            // Plane or chunk may not be loaded, since destruction proceeds top-down.
            if let Some(p) = w.planes.get_mut(s.plane) {
                let chunk_pos = s.pos.reduce().div_floor(scalar(CHUNK_SIZE));
                if let Some(&tcid) = p.loaded_chunks.get(&chunk_pos) {
                    if let Some(tc) = w.terrain_chunks.get_mut(tcid) {
                        tc.child_structures.remove(&sid);
                    }
                }
            }
        },
    }

    for &iid in s.child_inventories.iter() {
        ops::inventory::destroy(f, iid).unwrap();
    }

    f.with_hooks(|h| h.on_structure_destroy(sid, s.plane, bounds));
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
        // TODO: proper support for Plane attachment
        Plane => {},
        Chunk => {
            // Structures can exist only in planes that are currently loaded.
            let p = &w.planes[s.plane];
            let &tcid = unwrap!(p.loaded_chunks.get(&chunk_pos),
                                "can't attach structure to unloaded chunk");
            let tc = &mut w.terrain_chunks[tcid];
            tc.child_structures.insert(sid);
            // No more checks beyond this point.
        },
    }

    match old_attach {
        Plane => {},
        Chunk => {
            // If we're detaching from Chunk, we know the containing chunk is loaded because `c` is
            // loaded and has attachment Chunk.
            let p = &w.planes[s.plane];
            let tcid = p.loaded_chunks[&chunk_pos];
            let tc = &mut w.terrain_chunks[tcid];
            tc.child_structures.remove(&sid);
        },
    }

    s.attachment = new_attach;
    Ok(old_attach)
}

pub fn replace<'d, F>(f: &mut F,
                      sid: StructureId,
                      new_tid: TemplateId) -> OpResult<()>
        where F: Fragment<'d> {
    let (pid, pos, old_t, new_t) = {
        let w = f.world();
        let s = unwrap!(w.structures.get(sid));
        let old_t = unwrap!(w.data.structure_templates.get_template(s.template));
        let new_t = unwrap!(w.data.structure_templates.get_template(new_tid));
        (s.plane, s.pos, old_t, new_t)
    };

    let old_bounds = Region::new(pos, pos + old_t.size);
    let new_bounds = Region::new(pos, pos + new_t.size);

    if old_t.size != new_t.size ||
       old_t.shape != new_t.shape ||
       old_t.layer != new_t.layer {
        // If the templates aren't identical, we need to do some extra checks.
        if new_bounds.min.z < 0 || new_bounds.max.z > CHUNK_SIZE {
            fail!("structure replacement blocked by map bounds");
        }

        if !f.with_hooks(|h| h.check_structure_replacement(sid, new_t, pid, pos)) {
            fail!("structure replacement blocked by terrain or other structure");
        }
    }

    {
        let w = f.world_mut();
        let s = &mut w.structures[sid];
        s.template = new_tid;
    }

    f.with_hooks(|h| h.on_structure_replace(sid, pid, old_bounds));
    Ok(())
}

fn add_to_lookup(lookup: &mut HashMap<(PlaneId, V2), HashSet<StructureId>>,
                 sid: StructureId,
                 pid: PlaneId,
                 bounds: Region) {
    let chunk_bounds = bounds.reduce().div_round_signed(CHUNK_SIZE);
    for chunk_pos in chunk_bounds.points() {
        multimap_insert(lookup, (pid, chunk_pos), sid);
    }
}

fn remove_from_lookup(lookup: &mut HashMap<(PlaneId, V2), HashSet<StructureId>>,
                      sid: StructureId,
                      pid: PlaneId,
                      bounds: Region) {
    let chunk_bounds = bounds.reduce().div_round_signed(CHUNK_SIZE);
    for chunk_pos in chunk_bounds.points() {
        multimap_remove(lookup, (pid, chunk_pos), sid);
    }
}
