use std::collections::HashSet;
use std::mem::replace;

use physics::{CHUNK_SIZE, CHUNK_BITS};
use physics::v3::{Vn, V3, V2, scalar};

use input::InputBits;
use types::*;
use util::StrError;
use util::NO_STABLE_ID;
use view::ViewState;

use super::{World, Update};
use super::{Client, TerrainChunk, Entity, Structure, Inventory};
use super::{EntityAttachment, StructureAttachment, InventoryAttachment};

pub type OpResult<T> = Result<T, StrError>;


fn client_create(w: &mut World,
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

fn client_destroy(w: &mut World,
                  cid: ClientId) -> OpResult<()> {
    let c = unwrap!(w.clients.remove(cid));
    // Further lookup failures indicate an invariant violation.

    for &eid in c.child_entities.iter() {
        entity_destroy(w, eid).unwrap();
    }

    // TODO: clean up inventories

    Ok(())
}

fn client_set_pawn(w: &mut World,
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

fn client_clear_pawn(w: &mut World,
                     now: Time,
                     cid: ClientId) -> OpResult<Option<EntityId>> {
    let c = unwrap!(w.clients.get_mut(cid));
    // NB: Keep this behavior in sync with entity_destroy.
    let old_eid = replace(&mut c.pawn, None);
    Ok(old_eid)
}


fn terrain_chunk_create(w: &mut World,
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

fn terrain_chunk_destroy(w: &mut World,
                         pos: V2) -> OpResult<()> {
    let ok = w.terrain_chunks.remove(&pos).is_some();
    if !ok {
        fail!("no chunk exists with given position");
    }

    // TODO: remove entities and structures that have Chunk attachment

    Ok(())
}


fn entity_create(w: &mut World,
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

fn entity_destroy(w: &mut World,
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

fn entity_attach(w: &mut World,
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
            let c = w.clients.get_mut(cid).unwrap();
            c.child_entities.remove(&eid);
        },
    }

    Ok(old_attach)
}
