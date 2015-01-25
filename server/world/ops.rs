use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Hasher;
use std::hash::Hash;
use std::mem::replace;

use physics::{CHUNK_SIZE, CHUNK_BITS};
use physics::v3::{Vn, V3, V2, scalar, Region};

use input::InputBits;
use types::*;
use util::StrError;
use util::NO_STABLE_ID;
use view::ViewState;

use super::{World, Update};
use super::{Client, TerrainChunk, Entity, Structure, Inventory};
use super::{EntityAttachment, StructureAttachment, InventoryAttachment};

pub type OpResult<T> = Result<T, StrError>;


pub fn client_create(w: &mut World,
                     chunk_offset: (u8, u8)) -> OpResult<ClientId> {
    let c = Client {
        pawn: None,
        current_input: InputBits::empty(),
        chunk_offset: chunk_offset,
        view_state: ViewState::new(scalar(0)),

        stable_id: NO_STABLE_ID,
        child_entities: HashSet::new(),
        child_inventories: HashSet::new(),
    };

    Ok(w.clients.insert(c))
}

pub fn client_destroy(w: &mut World,
                      cid: ClientId) -> OpResult<()> {
    let c = unwrap!(w.clients.remove(cid));
    // Further lookup failures indicate an invariant violation.

    for &eid in c.child_entities.iter() {
        entity_destroy(w, eid).unwrap();
    }

    // TODO: clean up inventories

    Ok(())
}

pub fn client_set_pawn(w: &mut World,
                       now: Time,
                       cid: ClientId,
                       eid: EntityId) -> OpResult<Option<EntityId>> {
    try!(entity_attach(w, eid, EntityAttachment::Client(cid)));
    let old_eid;

    {
        let c = unwrap!(w.clients.get_mut(cid));
        let e = unwrap!(w.entities.get_mut(eid));

        old_eid = replace(&mut c.pawn, Some(eid));
        c.view_state = ViewState::new(e.pos(now));
    }

    w.record(Update::ClientViewReset(cid));

    Ok(old_eid)
}

pub fn client_clear_pawn(w: &mut World,
                         cid: ClientId) -> OpResult<Option<EntityId>> {
    let c = unwrap!(w.clients.get_mut(cid));
    // NB: Keep this behavior in sync with entity_destroy.
    let old_eid = replace(&mut c.pawn, None);
    Ok(old_eid)
}


pub fn terrain_chunk_create(w: &mut World,
                            pos: V2,
                            blocks: [BlockId; 1 << (CHUNK_BITS * 3)]) -> OpResult<()> {
    if w.terrain_chunks.contains_key(&pos) {
        fail!("chunk already exists with same position");
    }

    let tc = TerrainChunk {
        blocks: blocks,
    };

    w.terrain_chunks.insert(pos, tc);
    Ok(())
}

pub fn terrain_chunk_destroy(w: &mut World,
                             pos: V2) -> OpResult<()> {
    let ok = w.terrain_chunks.remove(&pos).is_some();
    if !ok {
        fail!("no chunk exists with given position");
    }

    // TODO: remove entities and structures that have Chunk attachment

    Ok(())
}


pub fn entity_create(w: &mut World,
                     pos: V3,
                     anim: AnimId) -> OpResult<EntityId> {
    let e = Entity {
        motion: super::Motion,
        anim: anim,
        facing: V3::new(1, 0, 0),

        stable_id: NO_STABLE_ID,
        attachment: EntityAttachment::World,
        child_inventories: HashSet::new(),
    };

    Ok(w.entities.insert(e))
}

pub fn entity_destroy(w: &mut World,
                      eid: EntityId) -> OpResult<()> {
    use super::EntityAttachment::*;
    let e = unwrap!(w.entities.remove(eid));
    // Further lookup failures indicate an invariant violation.

    match e.attachment {
        World => {},
        Chunk => {},
        Client(cid) => {
            if let Some(c) = w.clients.get_mut(cid) {
                if c.pawn == Some(eid) {
                    // NB: keep this behavior in sync with client_clear_pawn
                    c.pawn = None;
                }
                c.child_entities.remove(&eid);
            }
            // else, we are being called recursively from client_destroy, so there's no need to
            // update the parent client.
        },
    }

    // TODO: clean up inventories

    Ok(())
}

pub fn entity_attach(w: &mut World,
                     eid: EntityId,
                     new_attach: EntityAttachment) -> OpResult<EntityAttachment> {
    use super::EntityAttachment::*;

    let e = unwrap!(w.entities.get_mut(eid));

    if new_attach == e.attachment {
        return Ok(new_attach);
    }

    match new_attach {
        World => {},
        Chunk => {
            fail!("EntityAttachment::Chunk is not yet supported");
            // TODO: check that e.motion is stationary
            let chunk_id = e.pos(0).reduce().div_floor(scalar(CHUNK_SIZE));
            unwrap!(w.terrain_chunks.get(&chunk_id),
                    "can't attach entity to unloaded chunk");
            // NB: TerrainChunks don't have explicit "child" sets.  We use the regular
            // entities-by-position cache instead, and `e` should already be in that cache.
        },
        Client(cid) => {
            let c = unwrap!(w.clients.get_mut(cid),
                            "can't attach entity to nonexistent client");
            c.child_entities.insert(eid);
        },
    }

    let old_attach = replace(&mut e.attachment, new_attach);

    // For `old_attach`, we assume that the chunk/client/etc exists, due to the World invariants.
    match old_attach {
        World => {},
        Chunk => {},    // No separate cache to update
        Client(cid) => {
            let c = &mut w.clients[cid];
            c.child_entities.remove(&eid);
        },
    }

    Ok(old_attach)
}


pub fn structure_create(w: &mut World,
                        pos: V3,
                        tid: TemplateId) -> OpResult<StructureId> {
    let t = unwrap!(w.data.object_templates.get_template(tid));
    let bounds = Region::new(pos, pos + t.size);

    if !structure_check_placement(w, bounds) {
        fail!("structure placement blocked by terrain or other structure");
    }

    let chunk_pos = pos.reduce().div_floor(scalar(CHUNK_SIZE));
    let offset = pos - chunk_pos.extend(0) * scalar(CHUNK_SIZE);

    let s = Structure {
        pos: pos,
        offset: (offset.x as u8,
                 offset.y as u8,
                 offset.z as u8),
        template: tid,

        stable_id: NO_STABLE_ID,
        attachment: StructureAttachment::World,
        child_inventories: HashSet::new(),
    };

    let sid = w.structures.insert(s);
    structure_add_to_lookup(&mut w.structures_by_chunk, sid, bounds);
    Ok(sid)
}

pub fn structure_destroy(w: &mut World,
                         sid: StructureId) -> OpResult<()> {
    use super::StructureAttachment::*;
    let s = unwrap!(w.structures.remove(sid));

    let t = w.data.object_templates.template(s.template);
    let bounds = Region::new(s.pos, s.pos + t.size);
    structure_remove_from_lookup(&mut w.structures_by_chunk, sid, bounds);

    match s.attachment {
        World => {},
        Chunk => {},
    }

    // TODO: clean up inventories

    Ok(())
}

pub fn structure_move(w: &mut World,
                      sid: StructureId,
                      new_pos: V3) -> OpResult<()> {
    let (old_bounds, new_bounds) = {
        let s = unwrap!(w.structures.get(sid));
        let t = unwrap!(w.data.object_templates.get_template(s.template));

        (Region::new(s.pos, s.pos + t.size),
         Region::new(new_pos, new_pos + t.size))
    };

    structure_remove_from_lookup(&mut w.structures_by_chunk, sid, old_bounds);

    if structure_check_placement(w, new_bounds) {
        w.structures[sid].pos = new_pos;
        structure_add_to_lookup(&mut w.structures_by_chunk, sid, new_bounds);
        Ok(())
    } else {
        structure_add_to_lookup(&mut w.structures_by_chunk, sid, old_bounds);
        fail!("structure placement blocked by terrain or other structure");
    }
}

pub fn structure_replace(w: &mut World,
                         sid: StructureId,
                         new_tid: TemplateId) -> OpResult<()> {
    let (old_bounds, new_bounds) = {
        let s = unwrap!(w.structures.get_mut(sid));
        let old_t = unwrap!(w.data.object_templates.get_template(s.template));
        let new_t = unwrap!(w.data.object_templates.get_template(new_tid));

        (Region::new(s.pos, s.pos + old_t.size),
         Region::new(s.pos, s.pos + new_t.size))
    };

    structure_remove_from_lookup(&mut w.structures_by_chunk, sid, old_bounds);

    if structure_check_placement(w, new_bounds) {
        w.structures[sid].template = new_tid;
        structure_add_to_lookup(&mut w.structures_by_chunk, sid, new_bounds);
        Ok(())
    } else {
        structure_add_to_lookup(&mut w.structures_by_chunk, sid, old_bounds);
        fail!("structure placement blocked by terrain or other structure");
    }
}

fn structure_check_placement(w: &World,
                             bounds: Region) -> bool {
    let chunk_bounds = bounds.reduce().div_round_signed(CHUNK_SIZE);
    for chunk_pos in chunk_bounds.points() {
        if let Some(sids) = w.structures_by_chunk.get(&chunk_pos) {
            for &sid in sids.iter() {
                let other_bounds = w.structure(sid).bounds();
                if other_bounds.overlaps(bounds) {
                    return false;
                }
            }
        }

        /*
        if let Some(terrain) = w.terrain_chunks.get(&chunk_pos) {
            let chunk_region = Region::new(chunk_pos.extend(0),
                                           chunk_pos.extend(0) + scalar(CHUNK_SIZE));
            for point in bounds.intersect(chunk_region).points() {
                let idx = chunk_region.index(point);
                match terrain.blocks[idx]. {

                }
            }
        }
        */
    }
    true
}

fn structure_add_to_lookup(lookup: &mut HashMap<V2, HashSet<StructureId>>,
                           sid: StructureId,
                           bounds: Region) {
    let chunk_bounds = bounds.reduce().div_round_signed(CHUNK_SIZE);
    for chunk_pos in chunk_bounds.points() {
        multimap_insert(lookup, chunk_pos, sid);
    }
}

fn structure_remove_from_lookup(lookup: &mut HashMap<V2, HashSet<StructureId>>,
                                sid: StructureId,
                                bounds: Region) {
    let chunk_bounds = bounds.reduce().div_round_signed(CHUNK_SIZE);
    for chunk_pos in chunk_bounds.points() {
        multimap_remove(lookup, chunk_pos, sid);
    }
}

fn multimap_insert<K, V>(map: &mut HashMap<K, HashSet<V>>, k: K, v: V)
        where K: Hash<Hasher>+Eq,
              V: Hash<Hasher>+Eq {
    use std::collections::hash_map::Entry::*;
    let bucket = match map.entry(k) {
        Vacant(e) => e.insert(HashSet::new()),
        Occupied(e) => e.into_mut(),
    };
    bucket.insert(v);
}

fn multimap_remove<K, V>(map: &mut HashMap<K, HashSet<V>>, k: K, v: V)
        where K: Hash<Hasher>+Eq,
              V: Hash<Hasher>+Eq {
    use std::collections::hash_map::Entry::*;
    match map.entry(k) {
        Vacant(e) => panic!("bucket is already empty"),
        Occupied(mut e) => {
            e.get_mut().remove(&v);
            if e.get().is_empty() {
                e.remove();
            }
        },
    }
}
