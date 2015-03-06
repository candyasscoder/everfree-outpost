use std::borrow::ToOwned;
use std::collections::{HashMap, HashSet};
use std::mem::replace;

use physics::CHUNK_SIZE;
use physics::Shape;

use input::InputBits;
use types::*;
use util::StrError;
use util::stable_id_map::NO_STABLE_ID;
use util::{multimap_insert, multimap_remove};

use super::{World, Update};
use super::{Client, TerrainChunk, Entity, Structure, Inventory};
use super::{EntityAttachment, StructureAttachment, InventoryAttachment};
// Using a glob here causes name resolution errors.
#[allow(unused_imports)]
use super::object::{
    ObjectRefBase, ObjectRefMutBase,
    ClientRef, ClientRefMut,
    TerrainChunkRef, TerrainChunkRefMut,
    EntityRef, EntityRefMut,
    StructureRef, StructureRefMut,
};
use super::hooks::Hooks;

pub type OpResult<T> = Result<T, StrError>;


pub fn client_create<H>(w: &mut World,
                        h: &mut H,
                        name: &str,
                        chunk_offset: (u8, u8)) -> OpResult<ClientId>
        where H: Hooks {
    let c = Client {
        name: name.to_owned(),
        pawn: None,
        current_input: InputBits::empty(),
        chunk_offset: chunk_offset,

        stable_id: NO_STABLE_ID,
        child_entities: HashSet::new(),
        child_inventories: HashSet::new(),
    };

    let cid = unwrap!(w.clients.insert(c));
    w.record(Update::ClientCreated(cid));
    h.on_client_create(w, cid);
    Ok(cid)
}

pub fn client_create_unchecked<H>(w: &mut World,
                                  h: &mut H) -> ClientId
        where H: Hooks {
    let cid = w.clients.insert(Client {
        name: String::new(),
        pawn: None,
        current_input: InputBits::empty(),
        chunk_offset: (0, 0),

        stable_id: NO_STABLE_ID,
        child_entities: HashSet::new(),
        child_inventories: HashSet::new(),
    }).unwrap();     // Shouldn't fail when stable_id == NO_STABLE_ID
    w.record(Update::ClientCreated(cid));
    h.on_client_create(w, cid);
    cid
}

pub fn client_destroy<H>(w: &mut World,
                         h: &mut H,
                         cid: ClientId) -> OpResult<()>
        where H: Hooks {
    let c = unwrap!(w.clients.remove(cid));
    // Further lookup failures indicate an invariant violation.

    for &eid in c.child_entities.iter() {
        // TODO: do we really want .unwrap() here?
        entity_destroy(w, h, eid).unwrap();
    }

    for &iid in c.child_inventories.iter() {
        inventory_destroy(w, h, iid).unwrap();
    }

    w.record(Update::ClientDestroyed(cid));
    h.on_client_destroy(w, cid);
    Ok(())
}

pub fn client_set_pawn<H>(w: &mut World,
                          h: &mut H,
                          cid: ClientId,
                          eid: EntityId) -> OpResult<Option<EntityId>>
        where H: Hooks {
    try!(entity_attach(w, h, eid, EntityAttachment::Client(cid)));
    let old_eid;

    {
        let c = unwrap!(w.clients.get_mut(cid));
        // We know 'eid' is valid because the 'entity_attach' above succeeded.
        old_eid = replace(&mut c.pawn, Some(eid));
    }

    h.on_client_change_pawn(w, cid, old_eid, Some(eid));
    Ok(old_eid)
}

pub fn client_clear_pawn<H>(w: &mut World,
                            h: &mut H,
                            cid: ClientId) -> OpResult<Option<EntityId>>
        where H: Hooks {
    let old_eid;
    {
        let c = unwrap!(w.clients.get_mut(cid));
        // NB: Keep this behavior in sync with entity_destroy.
        old_eid = replace(&mut c.pawn, None);
    }

    w.record(Update::ClientPawnChange(cid));
    h.on_client_change_pawn(w, cid, old_eid, None);
    Ok(old_eid)
}


pub fn terrain_chunk_create<H>(w: &mut World,
                               h: &mut H,
                               pos: V2,
                               blocks: Box<BlockChunk>) -> OpResult<()>
        where H: Hooks {
    if w.terrain_chunks.contains_key(&pos) {
        fail!("chunk already exists with same position");
    }

    let tc = TerrainChunk {
        blocks: blocks,

        child_structures: HashSet::new(),
    };

    w.terrain_chunks.insert(pos, tc);
    w.record(Update::TerrainChunkCreated(pos));
    h.on_terrain_chunk_create(w, pos);
    Ok(())
}

pub fn terrain_chunk_destroy<H>(w: &mut World,
                                h: &mut H,
                                pos: V2) -> OpResult<()>
        where H: Hooks {
    let t = unwrap!(w.terrain_chunks.remove(&pos));

    for &sid in t.child_structures.iter() {
        structure_destroy(w, h, sid).unwrap();
    }

    w.record(Update::TerrainChunkDestroyed(pos));
    h.on_terrain_chunk_destroy(w, pos);
    Ok(())
}


pub fn entity_create<H>(w: &mut World,
                        h: &mut H,
                        pos: V3,
                        anim: AnimId,
                        appearance: u32) -> OpResult<EntityId>
        where H: Hooks {
    let e = Entity {
        motion: super::Motion::fixed(pos),
        anim: anim,
        facing: V3::new(1, 0, 0),
        target_velocity: scalar(0),
        appearance: appearance,

        stable_id: NO_STABLE_ID,
        attachment: EntityAttachment::World,
        child_inventories: HashSet::new(),
    };

    let eid = unwrap!(w.entities.insert(e));
    w.record(Update::EntityCreated(eid));
    h.on_entity_create(w, eid);
    Ok(eid)
}

pub fn entity_create_unchecked<H>(w: &mut World,
                                  h: &mut H) -> EntityId
        where H: Hooks {
    let eid = w.entities.insert(Entity {
        motion: super::Motion::fixed(scalar(0)),
        anim: 0,
        facing: scalar(0),
        target_velocity: scalar(0),
        appearance: 0,

        stable_id: NO_STABLE_ID,
        attachment: EntityAttachment::World,
        child_inventories: HashSet::new(),
    }).unwrap();     // Shouldn't fail when stable_id == NO_STABLE_ID
    w.record(Update::EntityCreated(eid));
    h.on_entity_create(w, eid);
    eid
}

pub fn entity_destroy<H>(w: &mut World,
                         h: &mut H,
                         eid: EntityId) -> OpResult<()>
        where H: Hooks {
    use super::EntityAttachment::*;
    let e = unwrap!(w.entities.remove(eid));
    // Further lookup failures indicate an invariant violation.

    match e.attachment {
        World => {},
        Chunk => {},
        Client(cid) => {
            // The parent Client may not exist due to `x_destroy` operating top-down.
            // (`client_destroy` destroys the Client first, then calls `entity_destroy` on each
            // child entity.  In this situation, `cid` will not be found in `w.clients`.)
            if let Some(c) = w.clients.get_mut(cid) {
                if c.pawn == Some(eid) {
                    // NB: keep this behavior in sync with client_clear_pawn
                    c.pawn = None;
                }
                c.child_entities.remove(&eid);
            }
        },
    }

    for &iid in e.child_inventories.iter() {
        inventory_destroy(w, h, iid).unwrap();
    }

    w.record(Update::EntityDestroyed(eid));
    h.on_entity_destroy(w, eid);
    Ok(())
}

pub fn entity_attach<H>(w: &mut World,
                        h: &mut H,
                        eid: EntityId,
                        new_attach: EntityAttachment) -> OpResult<EntityAttachment>
        where H: Hooks {
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
            /*
            let chunk_id = e.pos(0).reduce().div_floor(scalar(CHUNK_SIZE));
            unwrap!(w.terrain_chunks.get(&chunk_id),
                    "can't attach entity to unloaded chunk");
            */
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


pub fn structure_create<H>(w: &mut World,
                           h: &mut H,
                           pos: V3,
                           tid: TemplateId) -> OpResult<StructureId>
        where H: Hooks {
    let t = unwrap!(w.data.object_templates.get_template(tid));
    let bounds = Region::new(pos, pos + t.size);

    if !structure_check_placement(w, bounds) {
        fail!("structure placement blocked by terrain or other structure");
    }

    let s = Structure {
        pos: pos,
        template: tid,

        stable_id: NO_STABLE_ID,
        attachment: StructureAttachment::World,
        child_inventories: HashSet::new(),
    };

    let sid = unwrap!(w.structures.insert(s));
    structure_add_to_lookup(&mut w.structures_by_chunk, sid, bounds);
    invalidate_region(w, h, bounds);
    w.record(Update::StructureCreated(sid));
    h.on_structure_create(w, sid);
    Ok(sid)
}

pub fn structure_create_unchecked<H>(w: &mut World,
                                     h: &mut H) -> StructureId
        where H: Hooks {
    let sid = w.structures.insert(Structure {
        pos: scalar(0),
        template: 0,

        stable_id: NO_STABLE_ID,
        attachment: StructureAttachment::World,
        child_inventories: HashSet::new(),
    }).unwrap();     // Shouldn't fail when stable_id == NO_STABLE_ID
    w.record(Update::StructureCreated(sid));
    h.on_structure_create(w, sid);
    sid
}

pub fn structure_post_init<H>(w: &mut World,
                              h: &mut H,
                              sid: StructureId) -> OpResult<()>
        where H: Hooks {
    let bounds = {
        let s = unwrap!(w.structures.get(sid));
        let t = unwrap!(w.data.object_templates.get_template(s.template));

        Region::new(s.pos, s.pos + t.size)
    };

    structure_add_to_lookup(&mut w.structures_by_chunk, sid, bounds);
    invalidate_region(w, h, bounds);
    Ok(())
}

pub fn structure_pre_fini<H>(w: &mut World,
                             h: &mut H,
                             sid: StructureId) -> OpResult<()>
        where H: Hooks {
    let bounds = {
        let s = unwrap!(w.structures.get(sid));
        let t = unwrap!(w.data.object_templates.get_template(s.template));

        Region::new(s.pos, s.pos + t.size)
    };

    structure_remove_from_lookup(&mut w.structures_by_chunk, sid, bounds);
    invalidate_region(w, h, bounds);
    Ok(())
}

pub fn structure_destroy<H>(w: &mut World,
                            h: &mut H,
                            sid: StructureId) -> OpResult<()>
        where H: Hooks {
    use super::StructureAttachment::*;
    let s = unwrap!(w.structures.remove(sid));

    let t = w.data.object_templates.template(s.template);
    let bounds = Region::new(s.pos, s.pos + t.size);
    structure_remove_from_lookup(&mut w.structures_by_chunk, sid, bounds);
    invalidate_region(w, h, bounds);

    match s.attachment {
        World => {},
        Chunk => {
            let chunk_pos = s.pos.reduce().div_floor(scalar(CHUNK_SIZE));
            // Chunk may not be loaded, since destruction proceeds top-down.
            w.terrain_chunks.get_mut(&chunk_pos)
             .map(|t| t.child_structures.remove(&sid));
        },
    }

    for &iid in s.child_inventories.iter() {
        inventory_destroy(w, h, iid).unwrap();
    }

    w.record(Update::StructureDestroyed(sid));
    h.on_structure_destroy(w, sid);
    Ok(())
}

pub fn structure_attach<H>(w: &mut World,
                           h: &mut H,
                           sid: StructureId,
                           new_attach: StructureAttachment) -> OpResult<StructureAttachment>
        where H: Hooks {
    use super::StructureAttachment::*;

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

pub fn structure_move<H>(w: &mut World,
                         h: &mut H,
                         sid: StructureId,
                         new_pos: V3) -> OpResult<()>
        where H: Hooks {
    let (old_bounds, new_bounds) = {
        let s = unwrap!(w.structures.get_mut(sid));
        let t = unwrap!(w.data.object_templates.get_template(s.template));

        (Region::new(s.pos, s.pos + t.size),
         Region::new(new_pos, new_pos + t.size))
    };

    structure_remove_from_lookup(&mut w.structures_by_chunk, sid, old_bounds);

    if structure_check_placement(w, new_bounds) {
        {
            let s = &mut w.structures[sid];
            if s.attachment == StructureAttachment::Chunk {
                let old_chunk_pos = s.pos.reduce().div_floor(scalar(CHUNK_SIZE));
                let new_chunk_pos = new_pos.reduce().div_floor(scalar(CHUNK_SIZE));
                // The old chunk is loaded because `s` is loaded and has Chunk attachment.  The new
                // chunk is loaded because structure_check_placement requires all chunks
                // overlapping `new_bounds` to be loaded.
                w.terrain_chunks[old_chunk_pos].child_structures.remove(&sid);
                w.terrain_chunks[new_chunk_pos].child_structures.insert(sid);
            }
            s.pos = new_pos;
        }
        structure_add_to_lookup(&mut w.structures_by_chunk, sid, new_bounds);
        invalidate_region(w, h, old_bounds);
        invalidate_region(w, h, new_bounds);
        Ok(())
    } else {
        structure_add_to_lookup(&mut w.structures_by_chunk, sid, old_bounds);
        fail!("structure placement blocked by terrain or other structure");
    }
}

pub fn structure_replace<H>(w: &mut World,
                            h: &mut H,
                            sid: StructureId,
                            new_tid: TemplateId) -> OpResult<()>
        where H: Hooks {
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
        invalidate_region(w, h, old_bounds);
        invalidate_region(w, h, new_bounds);
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

        if let Some(chunk) = w.get_terrain_chunk(chunk_pos) {
            for point in bounds.intersect(chunk.bounds()).points() {
                match chunk.shape_at(point) {
                    Shape::Empty => {},
                    Shape::Floor if point.z == bounds.min.z => {},
                    _ => return false,
                }
            }
        } else {
            // Don't allow placing a structure into an unloaded chunk.
            return false;
        }
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

fn invalidate_region<H>(w: &mut World,
                        h: &mut H,
                        bounds: Region)
        where H: Hooks {
    let chunk_bounds = bounds.reduce().div_round_signed(CHUNK_SIZE);
    for chunk_pos in chunk_bounds.points() {
        w.record(Update::ChunkInvalidate(chunk_pos));
        h.on_chunk_invalidate(w, chunk_pos);
    }
}


pub fn inventory_create<H>(w: &mut World,
                           h: &mut H) -> OpResult<InventoryId>
        where H: Hooks {
    Ok(inventory_create_unchecked(w, h))
}

pub fn inventory_create_unchecked<H>(w: &mut World,
                                     h: &mut H) -> InventoryId
        where H: Hooks {
    let iid = w.inventories.insert(Inventory {
        contents: HashMap::new(),

        stable_id: NO_STABLE_ID,
        attachment: InventoryAttachment::World,
    }).unwrap();     // Shouldn't fail when stable_id == NO_STABLE_ID
    w.record(Update::InventoryCreated(iid));
    h.on_inventory_create(w, iid);
    iid
}

pub fn inventory_destroy<H>(w: &mut World,
                            h: &mut H,
                            iid: InventoryId) -> OpResult<()>
        where H: Hooks {
    use super::InventoryAttachment::*;
    let i = unwrap!(w.inventories.remove(iid));

    match i.attachment {
        World => {},
        Client(cid) => {
            if let Some(c) = w.clients.get_mut(cid) {
                c.child_inventories.remove(&iid);
            }
        },
        Entity(eid) => {
            if let Some(e) = w.entities.get_mut(eid) {
                e.child_inventories.remove(&iid);
            }
        },
        Structure(sid) => {
            if let Some(s) = w.structures.get_mut(sid) {
                s.child_inventories.remove(&iid);
            }
        },
    }

    w.record(Update::InventoryDestroyed(iid));
    h.on_inventory_destroy(w, iid);
    Ok(())
}

pub fn inventory_attach<H>(w: &mut World,
                           h: &mut H,
                           iid: InventoryId,
                           new_attach: InventoryAttachment) -> OpResult<InventoryAttachment>
        where H: Hooks {
    use super::InventoryAttachment::*;

    let i = unwrap!(w.inventories.get_mut(iid));

    if new_attach == i.attachment {
        return Ok(new_attach);
    }

    match new_attach {
        World => {},
        Client(cid) => {
            let c = unwrap!(w.clients.get_mut(cid),
                            "can't attach inventory to nonexistent client");
            c.child_inventories.insert(iid);
        },
        Entity(eid) => {
            let e = unwrap!(w.entities.get_mut(eid),
                            "can't attach inventory to nonexistent entity");
            e.child_inventories.insert(iid);
        },
        Structure(sid) => {
            let s = unwrap!(w.structures.get_mut(sid),
                            "can't attach inventory to nonexistent structure");
            s.child_inventories.insert(iid);
        },
    }

    let old_attach = replace(&mut i.attachment, new_attach);

    match old_attach {
        World => {},
        Client(cid) => {
            w.clients[cid].child_inventories.remove(&iid);
        },
        Entity(eid) => {
            w.entities[eid].child_inventories.remove(&iid);
        },
        Structure(sid) => {
            w.structures[sid].child_inventories.remove(&iid);
        },
    }

    Ok(old_attach)
}

/// Fails only if `iid` is not valid.
pub fn inventory_update<H>(w: &mut World,
                           h: &mut H,
                           iid: InventoryId,
                           item_id: ItemId,
                           adjust: i16) -> OpResult<u8>
        where H: Hooks {
    use std::collections::hash_map::Entry::*;

    let (old_value, new_value) = {
        let i = unwrap!(w.inventories.get_mut(iid));

        match i.contents.entry(item_id) {
            Vacant(e) => {
                let new_value = update_item_count(0, adjust);
                e.insert(new_value);
                (0, new_value)
            },
            Occupied(mut e) => {
                let old_value = *e.get();
                let new_value = update_item_count(old_value, adjust);
                if new_value == 0 {
                    e.remove();
                } else {
                    e.insert(new_value);
                }
                (old_value, new_value)
            },
        }
    };

    w.record(Update::InventoryUpdate(iid, item_id, old_value, new_value));
    h.on_inventory_update(w, iid, item_id, old_value, new_value);

    Ok(new_value)
}

fn update_item_count(old: u8, adjust: i16) -> u8 {
    use std::u8;
    let sum = old as i16 + adjust;
    if sum < u8::MIN as i16 {
        u8::MIN
    } else if sum > u8::MAX as i16 {
        u8::MAX
    } else {
        sum as u8
    }
}
